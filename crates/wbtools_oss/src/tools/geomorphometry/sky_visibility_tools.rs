use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::thread;

use chrono::{Datelike, FixedOffset, NaiveDate, NaiveTime, TimeZone};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, Rgba, RgbaImage};
use rand::RngExt;
use rayon::prelude::*;
use serde_json::json;
use wbprojection::{Crs, EpsgIdentifyPolicy, identify_epsg_from_wkt_with_policy};
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterFormat};
use wbvector::{Coord as VCoord, FieldDef, FieldType, FieldValue, Geometry, GeometryType, Layer, VectorFormat};

use crate::memory_store;
use crate::palettes::LegacyPalette;

pub struct HorizonAngleTool;
pub struct SkyViewFactorTool;
pub struct VisibilityIndexTool;
pub struct HorizonAreaTool;
pub struct AverageHorizonDistanceTool;
pub struct TimeInDaylightTool;
pub struct ShadowImageTool;
pub struct ShadowAnimationTool;
pub struct HypsometricallyTintedHillshadeTool;
pub struct TopoRenderTool;
pub struct SkylineAnalysisTool;

pub struct SkyVisibilityCore;

#[derive(Clone, Copy)]
struct Offset {
    x1: isize,
    y1: isize,
    x2: isize,
    y2: isize,
    w: f32,
    dist: f32,
    curv: f32,
}

impl SkyVisibilityCore {
    fn build_result(path: String) -> ToolRunResult {
        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(path));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    fn build_result_with_gif(output_locator: String, gif_locator: String) -> ToolRunResult {
        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("gif_path".to_string(), json!(gif_locator));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    fn write_output(output: &Raster, output_path: &str) -> Result<(), ToolError> {
        let output_format = RasterFormat::for_output_path(output_path)
            .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
        output
            .write(output_path, output_format)
            .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))
    }

    fn write_vector_output(layer: &Layer, output_path: &str) -> Result<(), ToolError> {
        let fmt = VectorFormat::detect(output_path)
            .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
        wbvector::write(layer, output_path, fmt)
            .map_err(|e| ToolError::Execution(format!("failed writing output vector: {e}")))
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

    fn parse_station_points(points_path: &str) -> Result<Vec<VCoord>, ToolError> {
        let layer = wbvector::read(points_path)
            .map_err(|e| ToolError::Execution(format!("failed reading points vector: {e}")))?;
        let mut stations = Vec::new();
        for feature in layer.iter() {
            match feature.geometry.as_ref() {
                Some(Geometry::Point(coord)) => stations.push(coord.clone()),
                Some(Geometry::MultiPoint(coords)) => stations.extend(coords.iter().cloned()),
                _ => {}
            }
        }
        if stations.is_empty() {
            return Err(ToolError::Validation(
                "parameter 'points' must contain at least one point geometry".to_string(),
            ));
        }
        Ok(stations)
    }

    fn radial_svg(width: f64, height: f64, values: &[f64], title: &str, stroke: &str) -> String {
        let cx = width * 0.5;
        let cy = height * 0.55;
        let radius = width.min(height) * 0.34;
        let max_val = values
            .iter()
            .copied()
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let mut points = String::new();
        for (i, value) in values.iter().enumerate() {
            let angle = (i as f64 / values.len().max(1) as f64) * std::f64::consts::TAU
                - std::f64::consts::FRAC_PI_2;
            let r = radius * (*value / max_val).clamp(0.0, 1.0);
            let x = cx + r * angle.cos();
            let y = cy + r * angle.sin();
            points.push_str(&format!("{:.2},{:.2} ", x, y));
        }
        format!(
            "<svg width='{:.0}' height='{:.0}' viewBox='0 0 {:.0} {:.0}' xmlns='http://www.w3.org/2000/svg'><rect width='100%' height='100%' fill='#fff'/><text x='{:.1}' y='28' text-anchor='middle' font-size='18' font-family='Helvetica,Arial,sans-serif' fill='#222'>{}</text><circle cx='{:.2}' cy='{:.2}' r='{:.2}' fill='none' stroke='#d9d9d9' stroke-width='1.5'/><circle cx='{:.2}' cy='{:.2}' r='{:.2}' fill='none' stroke='#ececec' stroke-width='1'/><polyline points='{}' fill='none' stroke='{}' stroke-width='2.5'/></svg>",
            width,
            height,
            width,
            height,
            cx,
            title,
            cx,
            cy,
            radius,
            cx,
            cy,
            radius * 0.5,
            points.trim_end(),
            stroke,
        )
    }

    fn parse_dem_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input"))
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'dem' has malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'dem' references unknown in-memory raster id '{}'",
                    id
                ))
            });
        }
        Raster::read(path).map_err(|e| ToolError::Execution(format!("Failed to read DEM: {}", e)))
    }

    fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
        if let Some(output_path) = output_path {
            if let Some(parent) = output_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ToolError::Execution(format!("Failed to create output directory: {}", e))
                    })?;
                }
            }
            let output_path_str = output_path.to_string_lossy().to_string();
            Self::write_output(&output, &output_path_str)?;
            Ok(output_path_str)
        } else {
            let id = memory_store::put_raster(output);
            Ok(memory_store::make_raster_memory_path(&id))
        }
    }

    fn parse_max_dist(args: &ToolArgs, default_inf: bool) -> f32 {
        let default = if default_inf { f32::INFINITY } else { 0.0 };
        args.get("max_dist")
            .or_else(|| args.get("maxdist"))
            .and_then(|v| {
                if let Some(n) = v.as_f64() {
                    return Some(n as f32);
                }
                let s = v.as_str()?;
                if s.eq_ignore_ascii_case("inf") || s.eq_ignore_ascii_case("infinity") {
                    Some(f32::INFINITY)
                } else {
                    s.parse::<f32>().ok()
                }
            })
            .unwrap_or(default)
    }

    fn num_threads() -> isize {
        match thread::available_parallelism() {
            Ok(n) => n.get() as isize,
            Err(_) => 1,
        }
    }

    fn diag_len(dem: &Raster) -> f32 {
        (((dem.y_max() - dem.y_min).powi(2) + (dem.x_max() - dem.x_min).powi(2)).sqrt()) as f32
    }

    fn clamp_max_dist(max_dist: f32, dem: &Raster, cell_size_x: f32) -> Result<f32, ToolError> {
        if max_dist <= 5.0 * cell_size_x {
            return Err(ToolError::Validation(
                "max_dist must be larger than 5x cell size".to_string(),
            ));
        }
        Ok(max_dist.min(Self::diag_len(dem)))
    }

    fn compute_offsets(
        azimuth: f32,
        max_dist: f32,
        cell_size_x: f32,
        cell_size_y: f32,
        use_curvature: bool,
    ) -> Vec<Offset> {
        let azimuth = azimuth % 360.0;
        let line_slope = if azimuth < 180.0 {
            ((90.0 - azimuth) as f64).to_radians().tan() as f32
        } else {
            ((270.0 - azimuth) as f64).to_radians().tan() as f32
        };

        let (x_step, y_step) = if azimuth >= 0.0 && azimuth <= 90.0 {
            (1, 1)
        } else if azimuth <= 180.0 {
            (1, -1)
        } else if azimuth <= 270.0 {
            (-1, -1)
        } else {
            (-1, 1)
        };

        let mut offsets: Vec<Offset> = Vec::new();

        if line_slope.abs() > 1e-10 {
            let mut y = 0.0_f32;
            loop {
                y += y_step as f32;
                let x = y / line_slope;
                let dist = (x * cell_size_x).hypot(-y * cell_size_y);
                if dist > max_dist {
                    break;
                }
                let x1 = x.floor() as isize;
                let x2 = x1 + 1;
                let y1 = -y as isize;
                let w = x - x1 as f32;
                let curv = if use_curvature {
                    dist.hypot(6_371_000.0_f32) - 6_371_000.0_f32
                } else {
                    0.0
                };
                offsets.push(Offset {
                    x1,
                    y1,
                    x2,
                    y2: y1,
                    w,
                    dist,
                    curv,
                });
            }
        }

        let mut x = 0.0_f32;
        loop {
            x += x_step as f32;
            let y = -(line_slope * x);
            let dist = (x * cell_size_x).hypot(y * cell_size_y);
            if dist > max_dist {
                break;
            }
            let y1 = y.floor() as isize;
            let y2 = y1 + 1;
            let x1 = x as isize;
            let w = y - y1 as f32;
            let curv = if use_curvature {
                dist.hypot(6_371_000.0_f32) - 6_371_000.0_f32
            } else {
                0.0
            };
            offsets.push(Offset {
                x1,
                y1,
                x2: x1,
                y2,
                w,
                dist,
                curv,
            });
        }

        offsets.sort_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap_or(Ordering::Equal));
        offsets
    }

    fn trace_horizon(
        dem: &Raster,
        row: isize,
        col: isize,
        offsets: &[Offset],
        nodata_f32: f32,
        observer_hgt_offset: f32,
        early_stop: bool,
    ) -> Option<(f32, f32)> {
        let mut current_elev = dem.get(0, row, col) as f32;
        if !current_elev.is_finite() || current_elev == nodata_f32 {
            return None;
        }
        current_elev += observer_hgt_offset;

        let early_stopping_slope = 80.0_f32.to_radians().tan();
        let a_small_value = -9_999_999.0_f32;
        let mut current_max_slope = a_small_value;
        let mut current_max_dist = 0.0_f32;

        for off in offsets {
            let x1 = col + off.x1;
            let y1 = row + off.y1;
            let x2 = col + off.x2;
            let y2 = row + off.y2;

            let mut z1 = dem.get(0, y1, x1) as f32;
            let mut z2 = dem.get(0, y2, x2) as f32;

            if z1 == nodata_f32 && z2 == nodata_f32 {
                break;
            } else if z1 == nodata_f32 {
                z1 = z2;
            } else if z2 == nodata_f32 {
                z2 = z1;
            }

            if !z1.is_finite() || !z2.is_finite() {
                break;
            }

            let z = z1 + off.w * (z2 - z1);
            let slope = ((z - off.curv) - current_elev) / off.dist;
            if slope > current_max_slope {
                current_max_slope = slope;
                current_max_dist = off.dist;
                if early_stop && slope > early_stopping_slope {
                    break;
                }
            }
        }

        if current_max_slope == a_small_value {
            Some((0.0, 0.0))
        } else {
            Some((current_max_slope, current_max_dist))
        }
    }

    fn horizon_angle_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "horizon_angle",
            display_name: "Horizon Angle",
            summary: "Calculates horizon angle (maximum slope) along a specified azimuth direction.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "azimuth",
                    description: "Azimuth in degrees [0, 360).",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn horizon_angle_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("azimuth".to_string(), json!(0.0));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("output".to_string(), json!("horizon_angle.tif"));

        let params = vec![
            ToolParamDescriptor {
                name: "dem".to_string(),
                description: "Input DEM raster path.".to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "azimuth".to_string(),
                description: "Azimuth in degrees [0, 360).".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "max_dist".to_string(),
                description: "Maximum search distance in map units.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "output".to_string(),
                description: "Output raster path.".to_string(),
                required: false,
            },
        ];

        let mut ex_args = ToolArgs::new();
        ex_args.insert("dem".to_string(), json!("dem.tif"));
        ex_args.insert("azimuth".to_string(), json!(315.0));
        ex_args.insert("output".to_string(), json!("horizon_angle.tif"));

        ToolManifest {
            id: "horizon_angle".to_string(),
            display_name: "Horizon Angle".to_string(),
            summary: "Calculates horizon angle (maximum slope) along a specified azimuth direction."
                .to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params,
            defaults,
            examples: vec![ToolExample {
                name: "basic_horizon_angle".to_string(),
                description: "Compute horizon angle at 315-degree azimuth.".to_string(),
                args: ex_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "horizon".to_string(),
            ],
            stability: ToolStability::Stable,
        }
    }

    fn validate_horizon_angle(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        Ok(())
    }

    fn run_horizon_angle(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let _ = DataType::F64;
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let azimuth = args
            .get("azimuth")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let mut max_dist = Self::parse_max_dist(args, true);

        let dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata_f32 = dem.nodata as f32;
        let cell_size_x = dem.cell_size_x as f32;
        let cell_size_y = dem.cell_size_y as f32;
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;

        let offsets = Arc::new(Self::compute_offsets(
            azimuth,
            max_dist,
            cell_size_x,
            cell_size_y,
            false,
        ));
        let dem = Arc::new(dem);

        let num_threads = Self::num_threads();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_threads {
            let tx = tx.clone();
            let dem = dem.clone();
            let offsets = offsets.clone();
            thread::spawn(move || {
                for row in (0..rows).filter(|r| r % num_threads == tid) {
                    let mut data = vec![nodata_f32 as f64; cols as usize];
                    for col in 0..cols {
                        if let Some((max_slope, _)) = SkyVisibilityCore::trace_horizon(
                            &dem,
                            row,
                            col,
                            &offsets,
                            nodata_f32,
                            0.0,
                            true,
                        ) {
                            data[col as usize] = max_slope.atan().to_degrees() as f64;
                        }
                    }
                    if tx.send((row, data)).is_err() {
                        return;
                    }
                }
            });
        }
        drop(tx);

        let mut output = (*dem).clone();
        for _ in 0..rows {
            let (row, data) = rx
                .recv()
                .map_err(|e| ToolError::Execution(format!("processing failed: {}", e)))?;
            output
                .set_row_slice(0, row, &data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn sky_view_factor_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "sky_view_factor",
            display_name: "Sky View Factor",
            summary: "Calculates the proportion of visible sky from a DEM/DSM.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM/DSM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "az_fraction",
                    description: "Azimuth step in degrees [1, 45].",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "observer_hgt_offset",
                    description: "Observer height offset above terrain.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn sky_view_factor_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dsm.tif"));
        defaults.insert("az_fraction".to_string(), json!(5.0));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("observer_hgt_offset".to_string(), json!(0.05));
        defaults.insert("output".to_string(), json!("svf.tif"));

        ToolManifest {
            id: "sky_view_factor".to_string(),
            display_name: "Sky View Factor".to_string(),
            summary: "Calculates the proportion of visible sky from a DEM/DSM.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate_sky_view_factor(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        let az_fraction = args
            .get("az_fraction")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0) as f32;
        if !(1.0..=45.0).contains(&az_fraction) {
            return Err(ToolError::Validation(
                "parameter 'az_fraction' must be in the range [1, 45]".to_string(),
            ));
        }
        Ok(())
    }

    fn run_sky_view_factor(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let az_fraction = args
            .get("az_fraction")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0) as f32;
        let mut max_dist = Self::parse_max_dist(args, true);
        let observer_hgt_offset = args
            .get("observer_hgt_offset")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05)
            .max(0.0) as f32;

        let dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;
        let cell_size_x = dem.cell_size_x as f32;
        let cell_size_y = dem.cell_size_y as f32;
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;

        let dem = Arc::new(dem);
        let mut sum = vec![0.0_f64; (rows * cols) as usize];
        let mut count = vec![0_u16; (rows * cols) as usize];

        let mut azimuth = 0.0_f32;
        while azimuth < 360.0 {
            let offsets = Arc::new(Self::compute_offsets(
                azimuth,
                max_dist,
                cell_size_x,
                cell_size_y,
                true,
            ));
            let num_threads = Self::num_threads();
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_threads {
                let tx = tx.clone();
                let dem = dem.clone();
                let offsets = offsets.clone();
                thread::spawn(move || {
                    for row in (0..rows).filter(|r| r % num_threads == tid) {
                        let mut data = vec![nodata; cols as usize];
                        let mut n = vec![0_u8; cols as usize];
                        for col in 0..cols {
                            if let Some((max_slope, _)) = SkyVisibilityCore::trace_horizon(
                                &dem,
                                row,
                                col,
                                &offsets,
                                nodata_f32,
                                observer_hgt_offset,
                                true,
                            ) {
                                data[col as usize] = max_slope.atan().sin().max(0.0) as f64;
                                n[col as usize] = 1;
                            }
                        }
                        if tx.send((row, data, n)).is_err() {
                            return;
                        }
                    }
                });
            }
            drop(tx);

            for _ in 0..rows {
                let (row, data, n) = rx
                    .recv()
                    .map_err(|e| ToolError::Execution(format!("processing failed: {}", e)))?;
                for col in 0..cols {
                    let idx = (row * cols + col) as usize;
                    if data[col as usize] != nodata {
                        sum[idx] += data[col as usize];
                        count[idx] = count[idx].saturating_add(n[col as usize] as u16);
                    }
                }
            }

            azimuth += az_fraction;
        }

        let mut output = (*dem).clone();
        for row in 0..rows {
            let mut row_data = vec![nodata; cols as usize];
            for col in 0..cols {
                let idx = (row * cols + col) as usize;
                let z = dem.get(0, row, col);
                if z as f32 != nodata_f32 {
                    let n = count[idx] as f64;
                    row_data[col as usize] = if n > 0.0 {
                        (1.0 - (sum[idx] / n)).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                }
            }
            output
                .set_row_slice(0, row, &row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn visibility_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "visibility_index",
            display_name: "Visibility Index",
            summary: "Calculates a topography-based visibility index from sampled viewsheds.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "station_height",
                    description: "Observer height above ground.",
                    required: false,
                },
                ToolParamSpec {
                    name: "resolution_factor",
                    description: "Sampling resolution factor [1, 25].",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum visibility search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn visibility_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("station_height".to_string(), json!(2.0));
        defaults.insert("resolution_factor".to_string(), json!(8));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("output".to_string(), json!("visibility_index.tif"));

        ToolManifest {
            id: "visibility_index".to_string(),
            display_name: "Visibility Index".to_string(),
            summary: "Calculates a topography-based visibility index from sampled viewsheds.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate_visibility_index(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        let max_dist = Self::parse_max_dist(args, true);
        if !max_dist.is_infinite() && max_dist <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'max_dist' must be positive when specified".to_string(),
            ));
        }
        Ok(())
    }

    fn idx(row: isize, col: isize, cols: isize) -> usize {
        (row * cols + col) as usize
    }

    fn run_visibility_index(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let station_height = args
            .get("station_height")
            .or_else(|| args.get("height"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0)
            .max(0.0) as f32;

        let mut res_factor = args
            .get("resolution_factor")
            .or_else(|| args.get("res_factor"))
            .and_then(|v| v.as_i64())
            .unwrap_or(8) as isize;
        res_factor = res_factor.clamp(1, 25);

        let mut max_dist = Self::parse_max_dist(args, true);

        let dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;
        let cell_size_x = dem.cell_size_x as f32;
        let cell_size_y = dem.cell_size_y as f32;
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;
        let max_dist_sq = max_dist * max_dist;

        let dem = Arc::new(dem);
        let num_cells_tested = (rows as f64 / res_factor as f64).ceil()
            * (cols as f64 / res_factor as f64).ceil();

        let num_threads = Self::num_threads();
        let (tx, rx) = mpsc::channel();

        for tid in 0..num_threads {
            let dem = dem.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut local_counts = vec![0_u32; (rows * cols) as usize];
                let mut view_angle = vec![-32768.0_f32; (rows * cols) as usize];
                let mut max_view_angle = vec![-32768.0_f32; (rows * cols) as usize];

                let set_val = |arr: &mut [f32], r: isize, c: isize, v: f32| {
                    let i = SkyVisibilityCore::idx(r, c, cols);
                    arr[i] = v;
                };
                let get_val = |arr: &[f32], r: isize, c: isize| -> f32 {
                    arr[SkyVisibilityCore::idx(r, c, cols)]
                };

                for stn_row in (0..rows)
                    .filter(|r| r % res_factor == 0 && ((r / res_factor) % num_threads == tid))
                {
                    for stn_col in (0..cols).filter(|c| c % res_factor == 0) {
                        let stn_elev = dem.get(0, stn_row, stn_col) as f32;
                        if stn_elev == nodata_f32 {
                            continue;
                        }
                        let stn_z = stn_elev + station_height;

                        // Reset this working buffer once per station with a contiguous fill.
                        max_view_angle.fill(-32768.0_f32);

                        for row in 0..rows {
                            for col in 0..cols {
                                let z = dem.get(0, row, col) as f32;
                                if z == nodata_f32 {
                                    set_val(&mut view_angle, row, col, -32768.0);
                                    continue;
                                }
                                let dx = (col - stn_col) as f32 * cell_size_x;
                                let dy = (row - stn_row) as f32 * cell_size_y;
                                let dist_sq = dx * dx + dy * dy;
                                if dist_sq > max_dist_sq {
                                    set_val(&mut view_angle, row, col, -32768.0);
                                    continue;
                                }
                                let dist = dist_sq.sqrt();
                                if dist > 0.0 {
                                    set_val(&mut view_angle, row, col, (z - stn_z) / dist * 1000.0);
                                } else {
                                    set_val(&mut view_angle, row, col, 0.0);
                                }
                            }
                        }

                        for row in (stn_row - 1)..=(stn_row + 1) {
                            for col in (stn_col - 1)..=(stn_col + 1) {
                                if row >= 0 && row < rows && col >= 0 && col < cols {
                                    let v = get_val(&view_angle, row, col);
                                    set_val(&mut max_view_angle, row, col, v);
                                }
                            }
                        }

                        if stn_row > 0 {
                            let mut max_va = get_val(&view_angle, stn_row - 1, stn_col);
                            let mut row = stn_row - 2;
                            loop {
                                if row < 0 {
                                    break;
                                }
                                let va = get_val(&view_angle, row, stn_col);
                                if va > max_va {
                                    max_va = va;
                                    local_counts[Self::idx(row, stn_col, cols)] += 1;
                                }
                                set_val(&mut max_view_angle, row, stn_col, max_va);
                                if row == 0 {
                                    break;
                                }
                                row -= 1;
                            }
                        }

                        if stn_row + 1 < rows {
                            let mut max_va = get_val(&view_angle, stn_row + 1, stn_col);
                            for row in (stn_row + 2)..rows {
                                let va = get_val(&view_angle, row, stn_col);
                                if va > max_va {
                                    max_va = va;
                                    local_counts[Self::idx(row, stn_col, cols)] += 1;
                                }
                                set_val(&mut max_view_angle, row, stn_col, max_va);
                            }
                        }

                        if stn_col + 1 < cols {
                            let mut max_va = get_val(&view_angle, stn_row, stn_col + 1);
                            for col in (stn_col + 2)..cols {
                                let va = get_val(&view_angle, stn_row, col);
                                if va > max_va {
                                    max_va = va;
                                    local_counts[Self::idx(stn_row, col, cols)] += 1;
                                }
                                set_val(&mut max_view_angle, stn_row, col, max_va);
                            }
                        }

                        if stn_col > 0 {
                            let mut max_va = get_val(&view_angle, stn_row, stn_col - 1);
                            let mut col = stn_col - 2;
                            loop {
                                if col < 0 {
                                    break;
                                }
                                let va = get_val(&view_angle, stn_row, col);
                                if va > max_va {
                                    max_va = va;
                                    local_counts[Self::idx(stn_row, col, cols)] += 1;
                                }
                                set_val(&mut max_view_angle, stn_row, col, max_va);
                                if col == 0 {
                                    break;
                                }
                                col -= 1;
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        if stn_row > 1 {
                            let mut row = stn_row - 2;
                            loop {
                                vert_count += 1.0;
                                let mut horiz_count = 0.0_f32;
                                let max_col = stn_col + vert_count as isize;
                                for col in (stn_col + 1)..=max_col {
                                    if col >= cols {
                                        break;
                                    }
                                    let va = get_val(&view_angle, row, col);
                                    horiz_count += 1.0;
                                    let tva = if horiz_count != vert_count {
                                        let t1 = get_val(&max_view_angle, row + 1, col - 1);
                                        let t2 = get_val(&max_view_angle, row + 1, col);
                                        t2 + horiz_count / vert_count * (t1 - t2)
                                    } else {
                                        get_val(&max_view_angle, row + 1, col - 1)
                                    };
                                    if tva > va {
                                        set_val(&mut max_view_angle, row, col, tva);
                                    } else {
                                        set_val(&mut max_view_angle, row, col, va);
                                        local_counts[Self::idx(row, col, cols)] += 1;
                                    }
                                }
                                if row == 0 {
                                    break;
                                }
                                row -= 1;
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        if stn_row > 1 {
                            let mut row = stn_row - 2;
                            loop {
                                vert_count += 1.0;
                                let mut horiz_count = 0.0_f32;
                                let min_col = stn_col - vert_count as isize;
                                let mut col = stn_col - 1;
                                loop {
                                    if col < min_col || col < 0 {
                                        break;
                                    }
                                    let va = get_val(&view_angle, row, col);
                                    horiz_count += 1.0;
                                    let tva = if horiz_count != vert_count {
                                        let t1 = get_val(&max_view_angle, row + 1, col + 1);
                                        let t2 = get_val(&max_view_angle, row + 1, col);
                                        t2 + horiz_count / vert_count * (t1 - t2)
                                    } else {
                                        get_val(&max_view_angle, row + 1, col + 1)
                                    };
                                    if tva > va {
                                        set_val(&mut max_view_angle, row, col, tva);
                                    } else {
                                        set_val(&mut max_view_angle, row, col, va);
                                        local_counts[Self::idx(row, col, cols)] += 1;
                                    }
                                    if col == 0 {
                                        break;
                                    }
                                    col -= 1;
                                }
                                if row == 0 {
                                    break;
                                }
                                row -= 1;
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        for row in (stn_row + 2)..rows {
                            vert_count += 1.0;
                            let mut horiz_count = 0.0_f32;
                            let min_col = stn_col - vert_count as isize;
                            let mut col = stn_col - 1;
                            loop {
                                if col < min_col || col < 0 {
                                    break;
                                }
                                let va = get_val(&view_angle, row, col);
                                horiz_count += 1.0;
                                let tva = if horiz_count != vert_count {
                                    let t1 = get_val(&max_view_angle, row - 1, col + 1);
                                    let t2 = get_val(&max_view_angle, row - 1, col);
                                    t2 + horiz_count / vert_count * (t1 - t2)
                                } else {
                                    get_val(&max_view_angle, row - 1, col + 1)
                                };
                                if tva > va {
                                    set_val(&mut max_view_angle, row, col, tva);
                                } else {
                                    set_val(&mut max_view_angle, row, col, va);
                                    local_counts[Self::idx(row, col, cols)] += 1;
                                }
                                if col == 0 {
                                    break;
                                }
                                col -= 1;
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        for row in (stn_row + 2)..rows {
                            vert_count += 1.0;
                            let mut horiz_count = 0.0_f32;
                            let max_col = stn_col + vert_count as isize;
                            for col in (stn_col + 1)..=max_col {
                                if col >= cols {
                                    break;
                                }
                                let va = get_val(&view_angle, row, col);
                                horiz_count += 1.0;
                                let tva = if horiz_count != vert_count {
                                    let t1 = get_val(&max_view_angle, row - 1, col - 1);
                                    let t2 = get_val(&max_view_angle, row - 1, col);
                                    t2 + horiz_count / vert_count * (t1 - t2)
                                } else {
                                    get_val(&max_view_angle, row - 1, col - 1)
                                };
                                if tva > va {
                                    set_val(&mut max_view_angle, row, col, tva);
                                } else {
                                    set_val(&mut max_view_angle, row, col, va);
                                    local_counts[Self::idx(row, col, cols)] += 1;
                                }
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        for col in (stn_col + 2)..cols {
                            vert_count += 1.0;
                            let mut horiz_count = 0.0_f32;
                            let min_row = stn_row - vert_count as isize;
                            let mut row = stn_row - 1;
                            loop {
                                if row < min_row || row < 0 {
                                    break;
                                }
                                let va = get_val(&view_angle, row, col);
                                horiz_count += 1.0;
                                let tva = if horiz_count != vert_count {
                                    let t1 = get_val(&max_view_angle, row + 1, col - 1);
                                    let t2 = get_val(&max_view_angle, row, col - 1);
                                    t2 + horiz_count / vert_count * (t1 - t2)
                                } else {
                                    get_val(&max_view_angle, row + 1, col - 1)
                                };
                                if tva > va {
                                    set_val(&mut max_view_angle, row, col, tva);
                                } else {
                                    set_val(&mut max_view_angle, row, col, va);
                                    local_counts[Self::idx(row, col, cols)] += 1;
                                }
                                if row == 0 {
                                    break;
                                }
                                row -= 1;
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        for col in (stn_col + 2)..cols {
                            vert_count += 1.0;
                            let mut horiz_count = 0.0_f32;
                            let max_row = stn_row + vert_count as isize;
                            for row in (stn_row + 1)..=max_row {
                                if row >= rows {
                                    break;
                                }
                                let va = get_val(&view_angle, row, col);
                                horiz_count += 1.0;
                                let tva = if horiz_count != vert_count {
                                    let t1 = get_val(&max_view_angle, row - 1, col - 1);
                                    let t2 = get_val(&max_view_angle, row, col - 1);
                                    t2 + horiz_count / vert_count * (t1 - t2)
                                } else {
                                    get_val(&max_view_angle, row - 1, col - 1)
                                };
                                if tva > va {
                                    set_val(&mut max_view_angle, row, col, tva);
                                } else {
                                    set_val(&mut max_view_angle, row, col, va);
                                    local_counts[Self::idx(row, col, cols)] += 1;
                                }
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        if stn_col > 1 {
                            let mut col = stn_col - 2;
                            loop {
                                vert_count += 1.0;
                                let mut horiz_count = 0.0_f32;
                                let max_row = stn_row + vert_count as isize;
                                for row in (stn_row + 1)..=max_row {
                                    if row >= rows {
                                        break;
                                    }
                                    let va = get_val(&view_angle, row, col);
                                    horiz_count += 1.0;
                                    let tva = if horiz_count != vert_count {
                                        let t1 = get_val(&max_view_angle, row - 1, col + 1);
                                        let t2 = get_val(&max_view_angle, row, col + 1);
                                        t2 + horiz_count / vert_count * (t1 - t2)
                                    } else {
                                        get_val(&max_view_angle, row - 1, col + 1)
                                    };
                                    if tva > va {
                                        set_val(&mut max_view_angle, row, col, tva);
                                    } else {
                                        set_val(&mut max_view_angle, row, col, va);
                                        local_counts[Self::idx(row, col, cols)] += 1;
                                    }
                                }
                                if col == 0 {
                                    break;
                                }
                                col -= 1;
                            }
                        }

                        let mut vert_count = 1.0_f32;
                        if stn_col > 1 {
                            let mut col = stn_col - 2;
                            loop {
                                vert_count += 1.0;
                                let mut horiz_count = 0.0_f32;
                                let min_row = stn_row - vert_count as isize;
                                let mut row = stn_row - 1;
                                loop {
                                    if row < min_row || row < 0 {
                                        break;
                                    }
                                    let va = get_val(&view_angle, row, col);
                                    horiz_count += 1.0;
                                    let tva = if horiz_count != vert_count {
                                        let t1 = get_val(&max_view_angle, row + 1, col + 1);
                                        let t2 = get_val(&max_view_angle, row, col + 1);
                                        t2 + horiz_count / vert_count * (t1 - t2)
                                    } else {
                                        get_val(&max_view_angle, row + 1, col + 1)
                                    };
                                    if tva > va {
                                        set_val(&mut max_view_angle, row, col, tva);
                                    } else {
                                        set_val(&mut max_view_angle, row, col, va);
                                        local_counts[Self::idx(row, col, cols)] += 1;
                                    }
                                    if row == 0 {
                                        break;
                                    }
                                    row -= 1;
                                }
                                if col == 0 {
                                    break;
                                }
                                col -= 1;
                            }
                        }
                    }
                }

                let _ = tx.send(local_counts);
            });
        }
        drop(tx);

        let mut output_counts = vec![0.0_f64; (rows * cols) as usize];
        for _ in 0..num_threads {
            let data = rx
                .recv()
                .map_err(|e| ToolError::Execution(format!("processing failed: {}", e)))?;
            for (i, v) in data.iter().enumerate() {
                output_counts[i] += *v as f64;
            }
        }

        let mut output = (*dem).clone();
        for row in 0..rows {
            let mut row_data = vec![nodata; cols as usize];
            for col in 0..cols {
                let z = dem.get(0, row, col) as f32;
                if z != nodata_f32 {
                    let idx = Self::idx(row, col, cols);
                    row_data[col as usize] = output_counts[idx] / num_cells_tested;
                }
            }
            output
                .set_row_slice(0, row, &row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn horizon_area_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "horizon_area",
            display_name: "Horizon Area",
            summary: "Calculates area of the horizon polygon (hectares).",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM/DSM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "az_fraction",
                    description: "Azimuth step in degrees [1, 45].",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "observer_hgt_offset",
                    description: "Observer height offset above terrain.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn horizon_area_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dsm.tif"));
        defaults.insert("az_fraction".to_string(), json!(5.0));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("observer_hgt_offset".to_string(), json!(0.05));
        defaults.insert("output".to_string(), json!("horizon_area.tif"));

        ToolManifest {
            id: "horizon_area".to_string(),
            display_name: "Horizon Area".to_string(),
            summary: "Calculates area of the horizon polygon (hectares).".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate_horizon_area(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        Ok(())
    }

    fn run_horizon_area(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let az_fraction = args
            .get("az_fraction")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0) as f32;
        let mut max_dist = Self::parse_max_dist(args, true);
        let observer_hgt_offset = args
            .get("observer_hgt_offset")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05)
            .max(0.0) as f32;

        let dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;
        let cell_size_x = dem.cell_size_x as f32;
        let cell_size_y = dem.cell_size_y as f32;
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;

        let dem = Arc::new(dem);
        let mut area_sum = vec![nodata; (rows * cols) as usize];
        let mut prev_x = vec![nodata_f32; (rows * cols) as usize];
        let mut prev_y = vec![nodata_f32; (rows * cols) as usize];

        let mut azimuth = 0.0_f32;
        while azimuth <= 360.0 {
            let cos_az = (azimuth as f64).to_radians().cos() as f32;
            let sin_az = (azimuth as f64).to_radians().sin() as f32;
            let offsets = Arc::new(Self::compute_offsets(
                azimuth,
                max_dist,
                cell_size_x,
                cell_size_y,
                true,
            ));
            let num_threads = Self::num_threads();
            let (tx, rx) = mpsc::channel();

            for tid in 0..num_threads {
                let tx = tx.clone();
                let dem = dem.clone();
                let offsets = offsets.clone();
                thread::spawn(move || {
                    for row in (0..rows).filter(|r| r % num_threads == tid) {
                        let mut row_x = vec![nodata_f32; cols as usize];
                        let mut row_y = vec![nodata_f32; cols as usize];
                        for col in 0..cols {
                            if let Some((max_slope, dist)) = SkyVisibilityCore::trace_horizon(
                                &dem,
                                row,
                                col,
                                &offsets,
                                nodata_f32,
                                observer_hgt_offset,
                                false,
                            ) {
                                if max_slope != 0.0 || dist != 0.0 {
                                    row_x[col as usize] = dist * cos_az;
                                    row_y[col as usize] = dist * sin_az;
                                } else {
                                    row_x[col as usize] = 0.0;
                                    row_y[col as usize] = 0.0;
                                }
                            }
                        }
                        if tx.send((row, row_x, row_y)).is_err() {
                            return;
                        }
                    }
                });
            }
            drop(tx);

            for _ in 0..rows {
                let (row, row_x, row_y) = rx
                    .recv()
                    .map_err(|e| ToolError::Execution(format!("processing failed: {}", e)))?;
                for col in 0..cols {
                    let i = Self::idx(row, col, cols);
                    if row_x[col as usize] != nodata_f32 {
                        if prev_x[i] != nodata_f32 {
                            let x1 = prev_x[i] as f64;
                            let y1 = prev_y[i] as f64;
                            let x2 = row_x[col as usize] as f64;
                            let y2 = row_y[col as usize] as f64;
                            if area_sum[i] == nodata {
                                area_sum[i] = 0.0;
                            }
                            area_sum[i] += x1 * y2 - x2 * y1;
                        } else {
                            area_sum[i] = 0.0;
                        }
                        prev_x[i] = row_x[col as usize];
                        prev_y[i] = row_y[col as usize];
                    }
                }
            }

            azimuth += az_fraction;
        }

        let mut output = (*dem).clone();
        for row in 0..rows {
            let mut row_data = vec![nodata; cols as usize];
            for col in 0..cols {
                let i = Self::idx(row, col, cols);
                if area_sum[i] != nodata {
                    row_data[col as usize] = (area_sum[i] / 2.0) / 10_000.0;
                }
            }
            output
                .set_row_slice(0, row, &row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn average_horizon_distance_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "average_horizon_distance",
            display_name: "Average Horizon Distance",
            summary: "Calculates average distance to horizon across azimuth directions.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM/DSM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "az_fraction",
                    description: "Azimuth step in degrees [1, 45].",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "observer_hgt_offset",
                    description: "Observer height offset above terrain.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn average_horizon_distance_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dsm.tif"));
        defaults.insert("az_fraction".to_string(), json!(5.0));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("observer_hgt_offset".to_string(), json!(0.05));
        defaults.insert("output".to_string(), json!("avg_horizon_distance.tif"));

        ToolManifest {
            id: "average_horizon_distance".to_string(),
            display_name: "Average Horizon Distance".to_string(),
            summary: "Calculates average distance to horizon across azimuth directions.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn time_in_daylight_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "time_in_daylight",
            display_name: "Time In Daylight",
            summary: "Calculates the proportion of daytime each cell is illuminated (not in terrain/object shadow).",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM/DSM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "az_fraction",
                    description: "Azimuth bin size in degrees (0, 360).",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum horizon search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "latitude",
                    description: "Optional latitude override in degrees; otherwise inferred from DEM CRS.",
                    required: false,
                },
                ToolParamSpec {
                    name: "longitude",
                    description: "Optional longitude override in degrees; otherwise inferred from DEM CRS.",
                    required: false,
                },
                ToolParamSpec {
                    name: "utc_offset",
                    description: "Optional UTC offset string like UTC+00:00; if omitted, estimated from center longitude.",
                    required: false,
                },
                ToolParamSpec {
                    name: "start_day",
                    description: "Start day-of-year (1..366).",
                    required: false,
                },
                ToolParamSpec {
                    name: "end_day",
                    description: "End day-of-year (1..366).",
                    required: false,
                },
                ToolParamSpec {
                    name: "start_time",
                    description: "Start time HH:MM:SS or 'sunrise'.",
                    required: false,
                },
                ToolParamSpec {
                    name: "end_time",
                    description: "End time HH:MM:SS or 'sunset'.",
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

    fn time_in_daylight_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dsm.tif"));
        defaults.insert("az_fraction".to_string(), json!(5.0));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("start_day".to_string(), json!(1));
        defaults.insert("end_day".to_string(), json!(365));
        defaults.insert("start_time".to_string(), json!("sunrise"));
        defaults.insert("end_time".to_string(), json!("sunset"));
        defaults.insert("output".to_string(), json!("time_in_daylight.tif"));

        ToolManifest {
            id: "time_in_daylight".to_string(),
            display_name: "Time In Daylight".to_string(),
            summary: "Calculates the proportion of daytime each cell is illuminated (not in terrain/object shadow)."
                .to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
                "solar".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn shadow_image_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "shadow_image",
            display_name: "Shadow Image",
            summary: "Generates a terrain shadow intensity raster for a specified date, time, and location.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM/DSM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum horizon search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "date",
                    description: "Date in DD/MM/YYYY format.",
                    required: false,
                },
                ToolParamSpec {
                    name: "time",
                    description: "Local time in HH:MM or HH:MMAM/HH:MMPM format.",
                    required: false,
                },
                ToolParamSpec {
                    name: "location",
                    description: "Location string LAT/LON/UTC_OFFSET, e.g. 43.5448/-80.2482/-4.",
                    required: false,
                },
                ToolParamSpec {
                    name: "palette",
                    description: "Hypsometric palette name (e.g. soft, atlas, high_relief, turbo, viridis, dem, grey, white).",
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

    fn shadow_image_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dsm.tif"));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("date".to_string(), json!("21/06/2021"));
        defaults.insert("time".to_string(), json!("13:00"));
        defaults.insert("location".to_string(), json!("43.5448/-80.2482/-4"));
        defaults.insert("palette".to_string(), json!("soft"));
        defaults.insert("output".to_string(), json!("shadow_image.tif"));

        ToolManifest {
            id: "shadow_image".to_string(),
            display_name: "Shadow Image".to_string(),
            summary: "Generates a terrain shadow intensity raster for a specified date, time, and location.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
                "solar".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn parse_shadow_time(v: &str) -> Result<(u32, u32), ToolError> {
        let s = v.trim().to_lowercase();
        let is_pm = s.contains("pm");
        let is_am = s.contains("am");
        let s = s.replace("am", "").replace("pm", "");
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(ToolError::Validation(
                "time must be in HH:MM or HH:MMAM/HH:MMPM format".to_string(),
            ));
        }
        let mut hour = parts[0]
            .trim()
            .parse::<u32>()
            .map_err(|_| ToolError::Validation("failed parsing hour in time string".to_string()))?;
        let minute = parts[1]
            .trim()
            .parse::<u32>()
            .map_err(|_| ToolError::Validation("failed parsing minute in time string".to_string()))?;
        if minute > 59 {
            return Err(ToolError::Validation("minute must be in [0, 59]".to_string()));
        }
        if is_pm {
            if hour < 12 {
                hour += 12;
            }
        } else if is_am && hour == 12 {
            hour = 0;
        }
        if hour > 23 {
            return Err(ToolError::Validation("hour must be in [0, 23]".to_string()));
        }
        Ok((hour, minute))
    }

    fn parse_shadow_date(v: &str) -> Result<(u32, u32, i32), ToolError> {
        let parts: Vec<&str> = v.trim().split('/').collect();
        if parts.len() != 3 {
            return Err(ToolError::Validation(
                "date must be in DD/MM/YYYY format".to_string(),
            ));
        }
        let day = parts[0]
            .trim()
            .parse::<u32>()
            .map_err(|_| ToolError::Validation("failed parsing day in date string".to_string()))?;
        let month = parts[1]
            .trim()
            .parse::<u32>()
            .map_err(|_| ToolError::Validation("failed parsing month in date string".to_string()))?;
        let year = parts[2]
            .trim()
            .parse::<i32>()
            .map_err(|_| ToolError::Validation("failed parsing year in date string".to_string()))?;
        if NaiveDate::from_ymd_opt(year, month, day).is_none() {
            return Err(ToolError::Validation(
                "date does not represent a valid calendar day".to_string(),
            ));
        }
        Ok((day, month, year))
    }

    fn parse_shadow_location(v: &str) -> Result<(f64, f64, f64), ToolError> {
        let parts: Vec<&str> = v.trim().split('/').collect();
        if parts.len() != 3 {
            return Err(ToolError::Validation(
                "location must be formatted as LAT/LON/UTC_OFFSET".to_string(),
            ));
        }
        let latitude = parts[0]
            .trim()
            .parse::<f64>()
            .map_err(|_| ToolError::Validation("failed parsing latitude in location string".to_string()))?;
        let longitude = parts[1]
            .trim()
            .parse::<f64>()
            .map_err(|_| ToolError::Validation("failed parsing longitude in location string".to_string()))?;
        let utc_offset = parts[2]
            .trim()
            .parse::<f64>()
            .map_err(|_| ToolError::Validation("failed parsing UTC offset in location string".to_string()))?;

        if !(-90.0..=90.0).contains(&latitude) {
            return Err(ToolError::Validation("latitude must be in [-90, 90]".to_string()));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(ToolError::Validation("longitude must be in [-180, 180]".to_string()));
        }
        if !(-12.0..=12.0).contains(&utc_offset) {
            return Err(ToolError::Validation("UTC offset must be in [-12, 12]".to_string()));
        }

        Ok((latitude, longitude, utc_offset))
    }

    fn validate_shadow_image(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") && !args.contains_key("input") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        if let Some(name) = args.get("palette").and_then(|v| v.as_str()) {
            if LegacyPalette::from_name(name).is_none() {
                return Err(ToolError::Validation(format!(
                    "unsupported palette '{}'; supported: {}",
                    name,
                    LegacyPalette::supported_names().join(", ")
                )));
            }
        }
        if let Some(d) = args.get("date").and_then(|v| v.as_str()) {
            let _ = Self::parse_shadow_date(d)?;
        }
        if let Some(t) = args.get("time").and_then(|v| v.as_str()) {
            let _ = Self::parse_shadow_time(t)?;
        }
        if let Some(l) = args.get("location").and_then(|v| v.as_str()) {
            let _ = Self::parse_shadow_location(l)?;
        }
        Ok(())
    }

    fn run_shadow_image(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut max_dist = Self::parse_max_dist(args, true);
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
        let no_hyspo_tint = matches!(palette, LegacyPalette::White);

        let date_text = args
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("21/06/2021");
        let time_text = args
            .get("time")
            .and_then(|v| v.as_str())
            .unwrap_or("13:00");
        let location_text = args
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("43.5448/-80.2482/-4");

        let (day, month, year) = Self::parse_shadow_date(date_text)?;
        let (hour, minute) = Self::parse_shadow_time(time_text)?;
        let (latitude, longitude, utc_offset) = Self::parse_shadow_location(location_text)?;

        let dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for row in 0..rows {
            for col in 0..cols {
                let z = dem.get(0, row, col) as f32;
                if z == nodata_f32 {
                    continue;
                }
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }
        }
        if !min_z.is_finite() || !max_z.is_finite() {
            return Err(ToolError::Validation(
                "input DEM contains no valid cells".to_string(),
            ));
        }
        let range = (max_z - min_z).max(f32::EPSILON);

        let palette_vals = palette.get_palette();
        let palette_vals: Vec<(f32, f32, f32)> = palette_vals
            .into_iter()
            .map(|(r, g, b)| (r.clamp(0.0, 255.0), g.clamp(0.0, 255.0), b.clamp(0.0, 255.0)))
            .collect();
        let p_last = palette_vals.len().saturating_sub(1) as f32;

        let mut cell_size_x = dem.cell_size_x.abs() as f32;
        let mut cell_size_y = dem.cell_size_y.abs() as f32;
        let mut z_factor = 1.0_f32;
        if Self::raster_is_geographic(&dem) {
            let lat_rad = latitude.to_radians() as f32;
            let meters_per_deg = 111_320.0_f32;
            let scale = lat_rad.cos().abs().max(1.0e-8);
            cell_size_x *= meters_per_deg * scale;
            cell_size_y *= meters_per_deg * scale;
            z_factor = 1.0 / (meters_per_deg * scale);
        }
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;

        let tz_secs = (utc_offset * 3600.0).round() as i32;
        let tz = if tz_secs < 0 {
            FixedOffset::west_opt(-tz_secs)
        } else {
            FixedOffset::east_opt(tz_secs)
        }
        .ok_or_else(|| ToolError::Validation("invalid UTC offset".to_string()))?;

        let dt = tz
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .ok_or_else(|| ToolError::Validation("invalid date/time with UTC offset".to_string()))?;
        let pos = solar_pos(dt.timestamp_millis(), latitude, longitude);
        let azimuth = pos.azimuth.to_degrees() as f32;
        let altitude = pos.altitude.to_degrees() as f32;

        let offsets = Arc::new(Self::compute_offsets(
            azimuth,
            max_dist,
            cell_size_x,
            cell_size_y,
            false,
        ));

        let dem = Arc::new(dem);
        let resx = cell_size_x.max(f32::EPSILON) as f64;
        let resy = cell_size_y.max(f32::EPSILON) as f64;
        let sin_theta = altitude.to_radians().sin() as f64;
        let cos_theta = altitude.to_radians().cos() as f64;
        let shadow_light = 0.40_f64;
        let solar_diameter_half = 0.25_f32;

        let row_data: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut out_row = vec![nodata; cols as usize];
                for col in 0..cols {
                    let zc = dem.get(0, row, col) as f32;
                    if zc == nodata_f32 {
                        continue;
                    }

                    let z = |dr: isize, dc: isize| {
                        let v = dem.get(0, row + dr, col + dc) as f32;
                        if v == nodata_f32 {
                            (zc as f64 * z_factor as f64) as f32
                        } else {
                            (v as f64 * z_factor as f64) as f32
                        }
                    };
                    let z1 = z(-1, -1) as f64;
                    let z2 = z(-1, 0) as f64;
                    let z3 = z(-1, 1) as f64;
                    let z4 = z(0, -1) as f64;
                    let z6 = z(0, 1) as f64;
                    let z7 = z(1, -1) as f64;
                    let z8 = z(1, 0) as f64;
                    let z9 = z(1, 1) as f64;

                    let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * resx);
                    let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * resy);
                    let ts = (dzdx * dzdx + dzdy * dzdy).sqrt().max(0.00017);
                    let slope_term = ts / (1.0 + ts * ts).sqrt();

                    let mut aspect = if dzdx != 0.0 {
                        std::f64::consts::PI - (dzdy / dzdx).atan()
                            + (std::f64::consts::FRAC_PI_2 * (dzdx / dzdx.abs()))
                    } else {
                        std::f64::consts::PI
                    };
                    if !aspect.is_finite() {
                        aspect = std::f64::consts::PI;
                    }

                    let horizon_deg = if altitude > -6.0 {
                        if let Some((max_slope, _)) = SkyVisibilityCore::trace_horizon(
                            &dem,
                            row,
                            col,
                            &offsets,
                            nodata_f32,
                            0.0,
                            true,
                        ) {
                            max_slope.atan().to_degrees()
                        } else {
                            0.0
                        }
                    } else {
                        90.0
                    };

                    let mut shade = if horizon_deg >= altitude + solar_diameter_half {
                        shadow_light
                    } else {
                        1.0
                    };

                    let term2 = sin_theta / ts;
                    let term3 = cos_theta
                        * (((azimuth as f64 - 90.0).to_radians()) - aspect).sin();
                    let hillshade = (slope_term * (term2 - term3)).max(0.0);
                    shade *= 0.75 + hillshade * 0.25;

                    if no_hyspo_tint {
                        out_row[col as usize] = shade.clamp(0.0, 1.0);
                    } else {
                        let mut p = ((zc - min_z) / range).clamp(0.0, 1.0);
                        if !p.is_finite() {
                            p = 0.0;
                        }
                        let idxf = p * p_last;
                        let i0 = idxf.floor() as usize;
                        let i1 = (i0 + 1).min(palette_vals.len() - 1);
                        let t = (idxf - i0 as f32).clamp(0.0, 1.0);

                        let (r0, g0, b0) = palette_vals[i0];
                        let (r1, g1, b1) = palette_vals[i1];
                        let rf = (r0 + t * (r1 - r0)) as f64;
                        let gf = (g0 + t * (g1 - g0)) as f64;
                        let bf = (b0 + t * (b1 - b0)) as f64;

                        let r = (rf * shade).clamp(0.0, 255.0) as u32;
                        let g = (gf * shade).clamp(0.0, 255.0) as u32;
                        let b = (bf * shade).clamp(0.0, 255.0) as u32;
                        out_row[col as usize] = ((255u32 << 24) | (b << 16) | (g << 8) | r) as f64;
                    }
                }
                out_row
            })
            .collect();

        let mut output = (*dem).clone();
        if no_hyspo_tint {
            output.data_type = DataType::F32;
        } else {
            output.data_type = DataType::U32;
            output.nodata = 0.0;
        }
        for (r, row) in row_data.iter().enumerate() {
            output
                .set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            ctx.progress.progress((r + 1) as f64 / rows as f64);
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn shadow_animation_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "shadow_animation",
            display_name: "Shadow Animation",
            summary: "Creates an interactive HTML viewer and animated GIF showing terrain shadows throughout a day.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM/DSM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output HTML path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "palette",
                    description: "Hypsometric palette name (e.g. soft, atlas, high_relief, turbo, viridis, dem, grey, white).",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum horizon search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "date",
                    description: "Date in DD/MM/YYYY format.",
                    required: false,
                },
                ToolParamSpec {
                    name: "time_interval",
                    description: "Frame interval in minutes [1, 60].",
                    required: false,
                },
                ToolParamSpec {
                    name: "location",
                    description: "Location string LAT/LON/UTC_OFFSET, e.g. 43.5448/-80.2482/-4.",
                    required: false,
                },
                ToolParamSpec {
                    name: "image_height",
                    description: "Displayed image height in pixels.",
                    required: false,
                },
                ToolParamSpec {
                    name: "delay",
                    description: "Per-frame GIF delay in milliseconds.",
                    required: false,
                },
                ToolParamSpec {
                    name: "label",
                    description: "Optional label displayed in the viewer.",
                    required: false,
                },
            ],
        }
    }

    fn shadow_animation_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dsm.tif"));
        defaults.insert("output".to_string(), json!("shadow_animation.html"));
        defaults.insert("palette".to_string(), json!("soft"));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("date".to_string(), json!("21/06/2021"));
        defaults.insert("time_interval".to_string(), json!(30));
        defaults.insert("location".to_string(), json!("43.5448/-80.2482/-4"));
        defaults.insert("image_height".to_string(), json!(600));
        defaults.insert("delay".to_string(), json!(250));
        defaults.insert("label".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("dem".to_string(), json!("dsm.tif"));
        example.insert("output".to_string(), json!("shadow_animation.html"));
        example.insert("date".to_string(), json!("21/06/2021"));
        example.insert("location".to_string(), json!("43.5448/-80.2482/-4"));

        ToolManifest {
            id: "shadow_animation".to_string(),
            display_name: "Shadow Animation".to_string(),
            summary: "Creates an interactive HTML viewer and animated GIF showing terrain shadows throughout a day.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "basic_shadow_animation".to_string(),
                description: "Generate a day-long shadow animation from a DEM or DSM.".to_string(),
                args: example,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
                "solar".to_string(),
                "animation".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate_shadow_animation(args: &ToolArgs) -> Result<(), ToolError> {
        Self::validate_shadow_image(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(v) = args
            .get("time_interval")
            .or_else(|| args.get("interval"))
            .and_then(|v| v.as_u64())
        {
            if !(1..=60).contains(&v) {
                return Err(ToolError::Validation(
                    "time_interval must be in [1, 60] minutes".to_string(),
                ));
            }
        }
        if let Some(v) = args
            .get("image_height")
            .or_else(|| args.get("height"))
            .and_then(|v| v.as_u64())
        {
            if v < 50 {
                return Err(ToolError::Validation(
                    "image_height must be at least 50 pixels".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn run_shadow_animation(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_html = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| std::env::temp_dir().join("shadow_animation.html"));
        let mut max_dist = Self::parse_max_dist(args, true);
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
        let date_text = args
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("21/06/2021");
        let location_text = args
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("43.5448/-80.2482/-4");
        let time_interval = args
            .get("time_interval")
            .or_else(|| args.get("interval"))
            .and_then(|v| v.as_u64())
            .unwrap_or(30)
            .clamp(1, 60) as usize;
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
        let (day, month, year) = Self::parse_shadow_date(date_text)?;
        let (latitude, longitude, utc_offset) = Self::parse_shadow_location(location_text)?;

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

        let dem = Arc::new(Self::load_raster(&dem_path)?);
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;
        if rows <= 0 || cols <= 0 {
            return Err(ToolError::Validation("input DEM is empty".to_string()));
        }

        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for row in 0..rows {
            for col in 0..cols {
                let z = dem.get(0, row, col) as f32;
                if z == nodata_f32 {
                    continue;
                }
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }
        }
        if !min_z.is_finite() || !max_z.is_finite() {
            return Err(ToolError::Validation(
                "input DEM contains no valid cells".to_string(),
            ));
        }
        let elev_range = (max_z - min_z).max(f32::EPSILON);
        let mut cell_size_x = dem.cell_size_x.abs() as f32;
        let mut cell_size_y = dem.cell_size_y.abs() as f32;
        let mut z_factor = 1.0_f32;
        if Self::raster_is_geographic(&dem) {
            let lat_rad = latitude.to_radians() as f32;
            let meters_per_deg = 111_320.0_f32;
            let scale = lat_rad.cos().abs().max(1.0e-8);
            cell_size_x *= meters_per_deg * scale;
            cell_size_y *= meters_per_deg * scale;
            z_factor = 1.0 / (meters_per_deg * scale);
        }
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;

        let palette_vals = palette.get_palette();
        let palette_vals: Vec<(f32, f32, f32)> = palette_vals
            .into_iter()
            .map(|(r, g, b)| (r.clamp(0.0, 255.0), g.clamp(0.0, 255.0), b.clamp(0.0, 255.0)))
            .collect();
        let p_last = palette_vals.len().saturating_sub(1) as f32;

        let resx = cell_size_x.max(f32::EPSILON) as f64;
        let resy = cell_size_y.max(f32::EPSILON) as f64;
        let mut base_rgba = vec![[0u8; 4]; (rows * cols) as usize];
        let mut tan_slope = vec![0.00017_f64; (rows * cols) as usize];
        let mut aspect = vec![std::f64::consts::PI; (rows * cols) as usize];
        let prep_rows: Vec<Vec<([u8; 4], f64, f64)>> = (0..rows as usize)
            .into_par_iter()
            .map(|row_usize| {
                let row = row_usize as isize;
                let mut row_out = vec![([0u8; 4], 0.00017_f64, std::f64::consts::PI); cols as usize];
                for col in 0..cols {
                    let zc = dem.get(0, row, col) as f32;
                    if zc == nodata_f32 {
                        continue;
                    }
                    let p = ((zc - min_z) / elev_range).clamp(0.0, 1.0);
                    let idxf = p * p_last;
                    let i0 = idxf.floor() as usize;
                    let i1 = (i0 + 1).min(palette_vals.len() - 1);
                    let t = (idxf - i0 as f32).clamp(0.0, 1.0);
                    let (r0, g0, b0) = palette_vals[i0];
                    let (r1, g1, b1) = palette_vals[i1];
                    let rgba = [
                        (r0 + t * (r1 - r0)).round().clamp(0.0, 255.0) as u8,
                        (g0 + t * (g1 - g0)).round().clamp(0.0, 255.0) as u8,
                        (b0 + t * (b1 - b0)).round().clamp(0.0, 255.0) as u8,
                        255,
                    ];

                    let z = |dr: isize, dc: isize| {
                        let v = dem.get(0, row + dr, col + dc) as f32;
                        if v == nodata_f32 {
                            (zc as f64 * z_factor as f64) as f32
                        } else {
                            (v as f64 * z_factor as f64) as f32
                        }
                    };
                    let z1 = z(-1, -1) as f64;
                    let z2 = z(-1, 0) as f64;
                    let z3 = z(-1, 1) as f64;
                    let z4 = z(0, -1) as f64;
                    let z6 = z(0, 1) as f64;
                    let z7 = z(1, -1) as f64;
                    let z8 = z(1, 0) as f64;
                    let z9 = z(1, 1) as f64;
                    let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * resx);
                    let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * resy);
                    let ts = (dzdx * dzdx + dzdy * dzdy).sqrt().max(0.00017);
                    let mut asp = if dzdx != 0.0 {
                        std::f64::consts::PI - (dzdy / dzdx).atan()
                            + (std::f64::consts::FRAC_PI_2 * (dzdx / dzdx.abs()))
                    } else {
                        std::f64::consts::PI
                    };
                    if !asp.is_finite() {
                        asp = std::f64::consts::PI;
                    }
                    row_out[col as usize] = (rgba, ts, asp);
                }
                row_out
            })
            .collect();
        for row in 0..rows as usize {
            for col in 0..cols as usize {
                let idx = Self::idx(row as isize, col as isize, cols);
                let (rgba, ts, asp) = prep_rows[row][col];
                base_rgba[idx] = rgba;
                tan_slope[idx] = ts;
                aspect[idx] = asp;
            }
        }
        let base_rgba = Arc::new(base_rgba);
        let tan_slope = Arc::new(tan_slope);
        let aspect = Arc::new(aspect);

        let tz_secs = (utc_offset * 3600.0).round() as i32;
        let tz = if tz_secs < 0 {
            FixedOffset::west_opt(-tz_secs)
        } else {
            FixedOffset::east_opt(tz_secs)
        }
        .ok_or_else(|| ToolError::Validation("invalid UTC offset".to_string()))?;

        let file_out = File::create(&gif_path)
            .map_err(|e| ToolError::Execution(format!("failed creating GIF output: {e}")))?;
        let mut encoder = GifEncoder::new(BufWriter::new(file_out));
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| ToolError::Execution(format!("failed setting GIF repeat: {e}")))?;
        let delay = Delay::from_numer_denom_ms(delay_ms, 1);
        let width = ((image_height as f64) * (cols as f64 / rows as f64)).round().max(1.0) as usize;
        let dark_shadow = 0.28_f64;

        let push_dark_frame = |encoder: &mut GifEncoder<BufWriter<File>>| -> Result<(), ToolError> {
            let mut img = RgbaImage::new(cols as u32, rows as u32);
            for row in 0..rows {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let px = base_rgba[idx];
                    if px[3] == 0 {
                        img.put_pixel(col as u32, row as u32, Rgba([0, 0, 0, 0]));
                    } else {
                        img.put_pixel(
                            col as u32,
                            row as u32,
                            Rgba([
                                (px[0] as f64 * dark_shadow).round() as u8,
                                (px[1] as f64 * dark_shadow).round() as u8,
                                (px[2] as f64 * dark_shadow).round() as u8,
                                255,
                            ]),
                        );
                    }
                }
            }
            encoder
                .encode_frame(Frame::from_parts(img, 0, 0, delay))
                .map_err(|e| ToolError::Execution(format!("failed encoding GIF frame: {e}")))
        };
        push_dark_frame(&mut encoder)?;

        let frame_minutes: Vec<usize> = (0..1440).step_by(time_interval).collect();
        let shadow_light = 0.40_f64;
        let solar_diameter_half = 0.25_f32;
        let mut frames_written = 1usize;
        for (frame_idx, minute_of_day) in frame_minutes.iter().enumerate() {
            let hour = (minute_of_day / 60) as u32;
            let minute = (minute_of_day % 60) as u32;
            let dt = tz
                .with_ymd_and_hms(year, month, day, hour, minute, 0)
                .single()
                .ok_or_else(|| ToolError::Validation("invalid date/time with UTC offset".to_string()))?;
            let pos = solar_pos(dt.timestamp_millis(), latitude, longitude);
            let azimuth = pos.azimuth.to_degrees() as f32;
            let altitude = pos.altitude.to_degrees() as f32;
            if altitude <= 0.0 {
                continue;
            }

            let offsets = Arc::new(Self::compute_offsets(
                azimuth,
                max_dist,
                cell_size_x,
                cell_size_y,
                false,
            ));
            let sin_theta = altitude.to_radians().sin() as f64;
            let cos_theta = altitude.to_radians().cos() as f64;
            let img_rows: Vec<Vec<[u8; 4]>> = (0..rows)
                .into_par_iter()
                .map(|row| {
                    let mut out_row = vec![[0u8; 4]; cols as usize];
                    for col in 0..cols {
                        let idx = Self::idx(row, col, cols);
                        let base = base_rgba[idx];
                        if base[3] == 0 {
                            continue;
                        }

                        let horizon_deg = if let Some((max_slope, _)) = Self::trace_horizon(
                            &dem,
                            row,
                            col,
                            &offsets,
                            nodata_f32,
                            0.0,
                            true,
                        ) {
                            max_slope.atan().to_degrees()
                        } else {
                            0.0
                        };
                        let mut shade = if horizon_deg >= altitude + solar_diameter_half {
                            shadow_light
                        } else {
                            1.0
                        };
                        let ts = tan_slope[idx];
                        let term1 = ts / (1.0 + ts * ts).sqrt();
                        let term2 = sin_theta / ts;
                        let term3 = cos_theta
                            * (((azimuth as f64 - 90.0).to_radians()) - aspect[idx]).sin();
                        let hillshade = (term1 * (term2 - term3)).max(0.0);
                        shade *= 0.75 + hillshade * 0.25;
                        out_row[col as usize] = [
                            (base[0] as f64 * shade).round().clamp(0.0, 255.0) as u8,
                            (base[1] as f64 * shade).round().clamp(0.0, 255.0) as u8,
                            (base[2] as f64 * shade).round().clamp(0.0, 255.0) as u8,
                            255,
                        ];
                    }
                    out_row
                })
                .collect();

            let mut img = RgbaImage::new(cols as u32, rows as u32);
            for row in 0..rows {
                for col in 0..cols {
                    img.put_pixel(
                        col as u32,
                        row as u32,
                        Rgba(img_rows[row as usize][col as usize]),
                    );
                }
            }
            encoder
                .encode_frame(Frame::from_parts(img, 0, 0, delay))
                .map_err(|e| ToolError::Execution(format!("failed encoding GIF frame: {e}")))?;
            frames_written += 1;
            ctx.progress
                .progress((frame_idx + 1) as f64 / frame_minutes.len().max(1) as f64);
        }
        push_dark_frame(&mut encoder)?;
        frames_written += 1;

        if frames_written < 3 {
            return Err(ToolError::Execution(
                "shadow animation did not produce any daylight frames for the supplied date and location"
                    .to_string(),
            ));
        }

        Self::write_animation_html(
            &output_html,
            "Shadow Animation",
            "Shadow Animation",
            &label,
            &gif_name,
            width,
            image_height,
            &[
                ("Input DEM", dem_path.clone()),
                ("Date", date_text.to_string()),
                ("Location", location_text.to_string()),
                ("Interval", format!("{} minutes", time_interval)),
            ],
        )?;

        Ok(Self::build_result_with_gif(
            output_html.to_string_lossy().to_string(),
            gif_path.to_string_lossy().to_string(),
        ))
    }

    fn hypsometrically_tinted_hillshade_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "hypsometrically_tinted_hillshade",
            display_name: "Hypsometrically Tinted Hillshade",
            summary: "Creates a Swiss-style terrain rendering by blending multi-azimuth hillshade with hypsometric tinting and optional atmospheric haze.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "solar_altitude",
                    description: "Solar altitude in degrees [0, 90].",
                    required: false,
                },
                ToolParamSpec {
                    name: "hillshade_weight",
                    description: "Hillshade blending weight in [0, 1].",
                    required: false,
                },
                ToolParamSpec {
                    name: "brightness",
                    description: "Brightness control in [0, 1].",
                    required: false,
                },
                ToolParamSpec {
                    name: "atmospheric_effects",
                    description: "Atmospheric haze amount in [0, 1].",
                    required: false,
                },
                ToolParamSpec {
                    name: "palette",
                    description: "Palette name (e.g. atlas, high_relief, soft, viridis, dem, grey).",
                    required: false,
                },
                ToolParamSpec {
                    name: "reverse_palette",
                    description: "Reverse palette order.",
                    required: false,
                },
                ToolParamSpec {
                    name: "full_360_mode",
                    description: "Use 8-direction illumination instead of 4-direction mode.",
                    required: false,
                },
                ToolParamSpec {
                    name: "z_factor",
                    description: "Vertical exaggeration multiplier.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn hypsometrically_tinted_hillshade_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("solar_altitude".to_string(), json!(45.0));
        defaults.insert("hillshade_weight".to_string(), json!(0.5));
        defaults.insert("brightness".to_string(), json!(0.5));
        defaults.insert("atmospheric_effects".to_string(), json!(0.0));
        defaults.insert("palette".to_string(), json!("atlas"));
        defaults.insert("reverse_palette".to_string(), json!(false));
        defaults.insert("full_360_mode".to_string(), json!(false));
        defaults.insert("z_factor".to_string(), json!(1.0));
        defaults.insert(
            "output".to_string(),
            json!("hypsometrically_tinted_hillshade.tif"),
        );

        let params = vec![
            ToolParamDescriptor {
                name: "dem".to_string(),
                description: "Input DEM raster path.".to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "solar_altitude".to_string(),
                description: "Solar altitude in degrees [0, 90].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "hillshade_weight".to_string(),
                description: "Hillshade blending weight in [0, 1].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "brightness".to_string(),
                description: "Brightness control in [0, 1].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "atmospheric_effects".to_string(),
                description: "Atmospheric haze amount in [0, 1].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "palette".to_string(),
                description: "Palette name (atlas, high_relief, arid, soft, muted, viridis, dem, grey, white, etc.).".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "reverse_palette".to_string(),
                description: "Reverse palette order.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "full_360_mode".to_string(),
                description: "Use 8-direction illumination instead of 4-direction mode.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "z_factor".to_string(),
                description: "Vertical exaggeration multiplier.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "output".to_string(),
                description: "Output raster path.".to_string(),
                required: false,
            },
        ];

        let mut ex_args = ToolArgs::new();
        ex_args.insert("dem".to_string(), json!("dem.tif"));
        ex_args.insert("solar_altitude".to_string(), json!(45.0));
        ex_args.insert("hillshade_weight".to_string(), json!(0.5));
        ex_args.insert("brightness".to_string(), json!(0.5));
        ex_args.insert("atmospheric_effects".to_string(), json!(0.0));
        ex_args.insert("palette".to_string(), json!("atlas"));
        ex_args.insert(
            "output".to_string(),
            json!("hypsometrically_tinted_hillshade.tif"),
        );

        ToolManifest {
            id: "hypsometrically_tinted_hillshade".to_string(),
            display_name: "Hypsometrically Tinted Hillshade".to_string(),
            summary: "Creates a Swiss-style terrain rendering by blending multi-azimuth hillshade with hypsometric tinting and optional atmospheric haze.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params,
            defaults,
            examples: vec![ToolExample {
                name: "basic_hypsometrically_tinted_hillshade".to_string(),
                description: "Render a hypsometrically tinted hillshade from a DEM.".to_string(),
                args: ex_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "hillshade".to_string(),
                "rendering".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate_hypsometrically_tinted_hillshade(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") && !args.contains_key("input") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        if let Some(name) = args.get("palette").and_then(|v| v.as_str()) {
            if LegacyPalette::from_name(name).is_none() {
                return Err(ToolError::Validation(format!(
                    "unsupported palette '{}'; supported: {}",
                    name,
                    LegacyPalette::supported_names().join(", ")
                )));
            }
        }
        if let Some(altitude) = args
            .get("solar_altitude")
            .or_else(|| args.get("altitude"))
            .and_then(|v| v.as_f64())
        {
            if !(0.0..=90.0).contains(&altitude) {
                return Err(ToolError::Validation(
                    "solar_altitude must be in [0, 90]".to_string(),
                ));
            }
        }
        if let Some(v) = args
            .get("hillshade_weight")
            .or_else(|| args.get("hs_weight"))
            .and_then(|v| v.as_f64())
        {
            if !(0.0..=1.0).contains(&v) {
                return Err(ToolError::Validation(
                    "hillshade_weight must be in [0, 1]".to_string(),
                ));
            }
        }
        if let Some(v) = args.get("brightness").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&v) {
                return Err(ToolError::Validation(
                    "brightness must be in [0, 1]".to_string(),
                ));
            }
        }
        if let Some(v) = args
            .get("atmospheric_effects")
            .or_else(|| args.get("atmospheric"))
            .and_then(|v| v.as_f64())
        {
            if !(0.0..=1.0).contains(&v) {
                return Err(ToolError::Validation(
                    "atmospheric_effects must be in [0, 1]".to_string(),
                ));
            }
        }
        if let Some(v) = args.get("z_factor").and_then(|v| v.as_f64()) {
            if v <= 0.0 {
                return Err(ToolError::Validation(
                    "z_factor must be > 0".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn run_hypsometrically_tinted_hillshade(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let solar_altitude = args
            .get("solar_altitude")
            .or_else(|| args.get("altitude"))
            .and_then(|v| v.as_f64())
            .unwrap_or(45.0)
            .clamp(0.0, 90.0);
        let hillshade_weight = args
            .get("hillshade_weight")
            .or_else(|| args.get("hs_weight"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        let brightness = args
            .get("brightness")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        let atmospheric_effects = args
            .get("atmospheric_effects")
            .or_else(|| args.get("atmospheric"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let reverse_palette = args
            .get("reverse_palette")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let full_360_mode = args
            .get("full_360_mode")
            .or_else(|| args.get("multidirection360mode"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let palette_name = args
            .get("palette")
            .and_then(|v| v.as_str())
            .unwrap_or("atlas");
        let palette = LegacyPalette::from_name(palette_name).ok_or_else(|| {
            ToolError::Validation(format!(
                "unsupported palette '{}'; supported: {}",
                palette_name,
                LegacyPalette::supported_names().join(", ")
            ))
        })?;
        let mut palette_vals = palette.get_palette();
        if reverse_palette {
            palette_vals.reverse();
        }
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

        let dem = Arc::new(Self::load_raster(&dem_path)?);
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;

        let mut z_factor = args
            .get("z_factor")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .max(f64::EPSILON);
        if Self::raster_is_geographic(&dem) {
            let mid_lat = ((dem.y_min + dem.y_max()) * 0.5).to_radians();
            z_factor = 1.0 / (111_320.0 * mid_lat.cos().abs().max(1.0e-8));
        }

        let mut elev_min = f64::INFINITY;
        let mut elev_max = f64::NEG_INFINITY;
        for row in 0..rows {
            for col in 0..cols {
                let z = dem.get(0, row, col);
                if z == nodata {
                    continue;
                }
                elev_min = elev_min.min(z);
                elev_max = elev_max.max(z);
            }
        }
        if !elev_min.is_finite() || !elev_max.is_finite() {
            return Err(ToolError::Validation(
                "input DEM contains no valid cells".to_string(),
            ));
        }
        let elev_range = (elev_max - elev_min).max(f64::EPSILON);

        let altitude = solar_altitude.to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();
        let eight_grid_res = (dem.cell_size_x.abs() * 8.0).max(f64::EPSILON);
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let half_pi = std::f64::consts::PI / 2.0;

        let (azimuths, weights): (Vec<f64>, Vec<f64>) = if full_360_mode {
            (
                vec![
                    (0.0_f64 - 90.0_f64).to_radians(),
                    (45.0_f64 - 90.0_f64).to_radians(),
                    (90.0_f64 - 90.0_f64).to_radians(),
                    (135.0_f64 - 90.0_f64).to_radians(),
                    (180.0_f64 - 90.0_f64).to_radians(),
                    (225.0_f64 - 90.0_f64).to_radians(),
                    (270.0_f64 - 90.0_f64).to_radians(),
                    (315.0_f64 - 90.0_f64).to_radians(),
                ],
                vec![0.15, 0.125, 0.1, 0.05, 0.1, 0.125, 0.15, 0.20],
            )
        } else {
            (
                vec![
                    (225.0_f64 - 90.0_f64).to_radians(),
                    (270.0_f64 - 90.0_f64).to_radians(),
                    (315.0_f64 - 90.0_f64).to_radians(),
                    (360.0_f64 - 90.0_f64).to_radians(),
                ],
                vec![0.1, 0.4, 0.4, 0.1],
            )
        };

        let hs_rows: Vec<Vec<i16>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut row_out = vec![-32768_i16; cols as usize];
                for col in 0..cols {
                    let z = dem.get(0, row, col);
                    if z == nodata {
                        continue;
                    }

                    let z_scaled = z * z_factor;
                    let mut n = [0.0_f64; 8];
                    for i in 0..8 {
                        let zn = dem.get(0, row + dy[i], col + dx[i]);
                        n[i] = if zn == nodata { z_scaled } else { zn * z_factor };
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
                    let mut shade = 0.0_f64;
                    for i in 0..azimuths.len() {
                        let term3 = cos_theta * (azimuths[i] - aspect).sin();
                        shade += term1 * (term2 - term3) * weights[i];
                    }

                    shade *= 32767.0;
                    if shade < 0.0 {
                        shade = 0.0;
                    }
                    row_out[col as usize] = shade.round() as i16;
                }
                row_out
            })
            .collect();

        let mut histo = vec![0.0_f64; 32768];
        let mut histo_elev = vec![0.0_f64; 32768];
        let mut num_cells = 0.0_f64;
        let mut hs_data = vec![-32768_i16; (rows * cols) as usize];
        for row in 0..rows {
            for col in 0..cols {
                let idx = Self::idx(row, col, cols);
                let hs = hs_rows[row as usize][col as usize];
                hs_data[idx] = hs;
                if hs == -32768 {
                    continue;
                }
                histo[hs as usize] += 1.0;
                num_cells += 1.0;

                let elev = dem.get(0, row, col);
                let ebin = (((elev - elev_min) / elev_range) * 32767.0)
                    .round()
                    .clamp(0.0, 32767.0) as usize;
                histo_elev[ebin] += 1.0;
            }
        }
        if num_cells == 0.0 {
            return Err(ToolError::Validation(
                "input DEM contains no valid cells".to_string(),
            ));
        }

        let clip_percent = 0.005_f64;
        let mut new_min = 0_i16;
        let mut new_max = 32767_i16;

        let mut target_cell_num = num_cells * clip_percent;
        let mut sum = 0.0;
        for (i, v) in histo.iter().enumerate() {
            sum += *v;
            if sum >= target_cell_num {
                new_min = i as i16;
                break;
            }
        }

        target_cell_num = num_cells * 0.10 * brightness;
        sum = 0.0;
        for i in (0..32768).rev() {
            sum += histo[i];
            if sum >= target_cell_num {
                new_max = i as i16;
                break;
            }
        }

        let mut new_elev_min = elev_min;
        let mut new_elev_max = elev_max;
        target_cell_num = num_cells * clip_percent;
        sum = 0.0;
        for (i, v) in histo_elev.iter().enumerate() {
            sum += *v;
            if sum >= target_cell_num {
                new_elev_min = elev_min + (i as f64 / 32768.0) * elev_range;
                break;
            }
        }
        sum = 0.0;
        for i in (0..32768).rev() {
            sum += histo_elev[i];
            if sum >= target_cell_num {
                new_elev_max = elev_min + (i as f64 / 32768.0) * elev_range;
                break;
            }
        }
        let new_elev_range = (new_elev_max - new_elev_min).max(f64::EPSILON);
        let hs_range = ((new_max - new_min) as f64).max(1.0);

        let hs_data = Arc::new(hs_data);
        let relief_alpha = 1.0 - hillshade_weight;
        let atmospheric_alpha = atmospheric_effects;
        let row_data: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut rng = rand::rng();
                let mut out_row = vec![0.0_f64; cols as usize];
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let hs_val = hs_data[idx];
                    if hs_val == -32768 {
                        continue;
                    }

                    let elev = dem.get(0, row, col);
                    let elev_proportion = ((elev - new_elev_min) / new_elev_range).clamp(0.0, 1.0);
                    let idxf = elev_proportion * p_last;
                    let i0 = idxf.floor() as usize;
                    let i1 = (i0 + 1).min(palette_vals.len() - 1);
                    let t = (idxf - i0 as f64).clamp(0.0, 1.0);
                    let (r0, g0, b0) = palette_vals[i0];
                    let (r1, g1, b1) = palette_vals[i1];
                    let red_relief = r0 + t * (r1 - r0);
                    let green_relief = g0 + t * (g1 - g0);
                    let blue_relief = b0 + t * (b1 - b0);

                    let mut alpha3 = atmospheric_alpha * (1.0 - elev_proportion);
                    let mut hs_val_f64 = hs_val as f64;
                    if alpha3 > 0.001 {
                        alpha3 += (rng.random_range(0..400) as f64 / 1000.0) * alpha3;
                        alpha3 = alpha3.clamp(0.0, 1.0);

                        let mut smoothed_sum = 0.0_f64;
                        let mut count = 0.0_f64;
                        for dr in -2..=2 {
                            for dc in -2..=2 {
                                let rr = row + dr;
                                let cc = col + dc;
                                if rr < 0 || rr >= rows || cc < 0 || cc >= cols {
                                    continue;
                                }
                                let nidx = Self::idx(rr, cc, cols);
                                let hn = hs_data[nidx];
                                if hn == -32768 {
                                    continue;
                                }
                                smoothed_sum += hn as f64;
                                count += 1.0;
                            }
                        }
                        if count > 0.0 {
                            let smoothed_hs = smoothed_sum / count;
                            hs_val_f64 = hs_val_f64 * (1.0 - alpha3) + smoothed_hs * alpha3;
                        }
                    }

                    let mut hs_proportion = ((hs_val_f64 - new_min as f64) / hs_range).clamp(0.0, 1.0);
                    hs_proportion = relief_alpha + hillshade_weight * hs_proportion;

                    let prop_r = (1.0 * (1.0 - hs_proportion)) + red_relief * hs_proportion;
                    let prop_g = (25.0 * (1.0 - hs_proportion)) + green_relief * hs_proportion;
                    let prop_b = (50.0 * (1.0 - hs_proportion)) + blue_relief * hs_proportion;

                    let r = (prop_r * (1.0 - alpha3) + alpha3 * 185.0)
                        .round()
                        .clamp(0.0, 255.0) as u32;
                    let g = (prop_g * (1.0 - alpha3) + alpha3 * 220.0)
                        .round()
                        .clamp(0.0, 255.0) as u32;
                    let b = (prop_b * (1.0 - alpha3) + alpha3 * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u32;
                    out_row[col as usize] = ((255u32 << 24) | (b << 16) | (g << 8) | r) as f64;
                }
                out_row
            })
            .collect();

        let mut output = (*dem).clone();
        output.data_type = DataType::U32;
        output.nodata = 0.0;
        for (r, row) in row_data.iter().enumerate() {
            output
                .set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            ctx.progress.progress((r + 1) as f64 / rows as f64);
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn topo_render_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "topo_render",
            display_name: "Topo Render",
            summary: "Creates a pseudo-3D topographic rendering using hypsometric tint, hillshade, horizon-based shadowing, and distance attenuation.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "palette",
                    description: "Palette name (e.g. soft, atlas, high_relief, turbo, viridis, dem, grey, white).",
                    required: false,
                },
                ToolParamSpec {
                    name: "reverse_palette",
                    description: "Reverse palette order.",
                    required: false,
                },
                ToolParamSpec {
                    name: "azimuth",
                    description: "Light-source azimuth in degrees [0, 360].",
                    required: false,
                },
                ToolParamSpec {
                    name: "altitude",
                    description: "Light-source altitude in degrees [0, 90].",
                    required: false,
                },
                ToolParamSpec {
                    name: "clipping_polygon",
                    description: "Optional polygon vector path; only DEM cells inside polygon(s) are rendered.",
                    required: false,
                },
                ToolParamSpec {
                    name: "background_hgt_offset",
                    description: "Vertical offset from minimum DEM elevation to background plane.",
                    required: false,
                },
                ToolParamSpec {
                    name: "background_clr",
                    description: "Background RGBA colour as array [r,g,b,a] with each channel in [0,255].",
                    required: false,
                },
                ToolParamSpec {
                    name: "attenuation_parameter",
                    description: "Distance attenuation exponent (>= 0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "ambient_light",
                    description: "Ambient light amount in [0, 1].",
                    required: false,
                },
                ToolParamSpec {
                    name: "z_factor",
                    description: "Vertical exaggeration multiplier.",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum shadow search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn topo_render_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("palette".to_string(), json!("soft"));
        defaults.insert("reverse_palette".to_string(), json!(false));
        defaults.insert("azimuth".to_string(), json!(315.0));
        defaults.insert("altitude".to_string(), json!(30.0));
        defaults.insert("background_hgt_offset".to_string(), json!(10.0));
        defaults.insert("background_clr".to_string(), json!([255, 255, 255, 255]));
        defaults.insert("attenuation_parameter".to_string(), json!(0.3));
        defaults.insert("ambient_light".to_string(), json!(0.2));
        defaults.insert("z_factor".to_string(), json!(1.0));
        defaults.insert("output".to_string(), json!("topo_render.tif"));

        let params = vec![
            ToolParamDescriptor {
                name: "dem".to_string(),
                description: "Input DEM raster path.".to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "palette".to_string(),
                description: "Palette name (soft, atlas, high_relief, turbo, viridis, dem, grey, white).".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "reverse_palette".to_string(),
                description: "Reverse palette order.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "azimuth".to_string(),
                description: "Light-source azimuth in degrees [0, 360].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "altitude".to_string(),
                description: "Light-source altitude in degrees [0, 90].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "clipping_polygon".to_string(),
                description: "Optional polygon vector path; only DEM cells inside polygon(s) are rendered.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "background_hgt_offset".to_string(),
                description: "Vertical offset from minimum DEM elevation to background plane.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "background_clr".to_string(),
                description: "Background RGBA colour as array [r,g,b,a].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "attenuation_parameter".to_string(),
                description: "Distance attenuation exponent (>= 0).".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "ambient_light".to_string(),
                description: "Ambient light amount in [0, 1].".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "z_factor".to_string(),
                description: "Vertical exaggeration multiplier.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "max_dist".to_string(),
                description: "Maximum shadow search distance in map units.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "output".to_string(),
                description: "Output raster path.".to_string(),
                required: false,
            },
        ];

        let mut ex_args = ToolArgs::new();
        ex_args.insert("dem".to_string(), json!("dem.tif"));
        ex_args.insert("palette".to_string(), json!("soft"));
        ex_args.insert("azimuth".to_string(), json!(315.0));
        ex_args.insert("altitude".to_string(), json!(30.0));
        ex_args.insert("output".to_string(), json!("topo_render.tif"));

        ToolManifest {
            id: "topo_render".to_string(),
            display_name: "Topo Render".to_string(),
            summary: "Creates a pseudo-3D topographic rendering using palette tinting, hillshade, shadows, and attenuation.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params,
            defaults,
            examples: vec![ToolExample {
                name: "basic_topo_render".to_string(),
                description: "Generate a pseudo-3D topographic render from a DEM.".to_string(),
                args: ex_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "rendering".to_string(),
                "topographic".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn parse_rgba_arg(args: &ToolArgs, key: &str, default: [u8; 4]) -> Result<[u8; 4], ToolError> {
        let Some(v) = args.get(key) else {
            return Ok(default);
        };

        let Some(arr) = v.as_array() else {
            return Err(ToolError::Validation(format!(
                "parameter '{}' must be a 4-element array [r,g,b,a]",
                key
            )));
        };
        if arr.len() != 4 {
            return Err(ToolError::Validation(format!(
                "parameter '{}' must have exactly 4 values",
                key
            )));
        }

        let mut out = [0u8; 4];
        for (i, ch) in arr.iter().enumerate() {
            let n = ch.as_i64().ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' contains a non-integer channel at index {}",
                    key, i
                ))
            })?;
            if !(0..=255).contains(&n) {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' channel {} must be in [0,255]",
                    key, i
                )));
            }
            out[i] = n as u8;
        }
        Ok(out)
    }

    fn validate_topo_render(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") && !args.contains_key("input") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        if let Some(name) = args.get("palette").and_then(|v| v.as_str()) {
            if LegacyPalette::from_name(name).is_none() {
                return Err(ToolError::Validation(format!(
                    "unsupported palette '{}'; supported: {}",
                    name,
                    LegacyPalette::supported_names().join(", ")
                )));
            }
        }
        if args.contains_key("clipping_polygon") || args.contains_key("polygon") {
            let _ = parse_vector_path_arg(args, "clipping_polygon")
                .or_else(|_| parse_vector_path_arg(args, "polygon"))?;
        }
        if let Some(az) = args.get("azimuth").and_then(|v| v.as_f64()) {
            if !(0.0..=360.0).contains(&az) {
                return Err(ToolError::Validation(
                    "azimuth must be in [0, 360]".to_string(),
                ));
            }
        }
        if let Some(alt) = args.get("altitude").and_then(|v| v.as_f64()) {
            if !(0.0..=90.0).contains(&alt) {
                return Err(ToolError::Validation(
                    "altitude must be in [0, 90]".to_string(),
                ));
            }
        }
        if let Some(a) = args.get("ambient_light").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&a) {
                return Err(ToolError::Validation(
                    "ambient_light must be in [0, 1]".to_string(),
                ));
            }
        }
        if let Some(a) = args
            .get("attenuation_parameter")
            .and_then(|v| v.as_f64())
        {
            if a < 0.0 {
                return Err(ToolError::Validation(
                    "attenuation_parameter must be >= 0".to_string(),
                ));
            }
        }
        if let Some(z) = args.get("z_factor").and_then(|v| v.as_f64()) {
            if z <= 0.0 {
                return Err(ToolError::Validation(
                    "z_factor must be > 0".to_string(),
                ));
            }
        }
        let _ = Self::parse_rgba_arg(args, "background_clr", [255, 255, 255, 255])?;
        Ok(())
    }

    fn run_topo_render(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

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
        let reverse_palette = args
            .get("reverse_palette")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let azimuth = args
            .get("azimuth")
            .and_then(|v| v.as_f64())
            .unwrap_or(315.0) as f32;
        let altitude = args
            .get("altitude")
            .and_then(|v| v.as_f64())
            .unwrap_or(30.0) as f32;
        let background_hgt_offset = args
            .get("background_hgt_offset")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0) as f32;
        let background_clr = Self::parse_rgba_arg(args, "background_clr", [255, 255, 255, 255])?;
        let attenuation_parameter = args
            .get("attenuation_parameter")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3) as f32;
        let ambient_light = args
            .get("ambient_light")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.2) as f32;
        let z_factor = args
            .get("z_factor")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let clipping_polygon_path = parse_vector_path_arg(args, "clipping_polygon")
            .or_else(|_| parse_vector_path_arg(args, "polygon"))
            .ok();

        let mut prepared_polys = Vec::<wbtopology::PreparedPolygon>::new();
        if let Some(poly_path) = clipping_polygon_path {
            let geoms = wbtopology::vector_io::read_geometries(&poly_path).map_err(|e| {
                ToolError::Validation(format!(
                    "failed reading clipping polygon vector '{}': {}",
                    poly_path, e
                ))
            })?;
            for g in geoms {
                if let wbtopology::Geometry::Polygon(poly) = g {
                    prepared_polys.push(wbtopology::PreparedPolygon::new(poly));
                }
            }
            if prepared_polys.is_empty() {
                return Err(ToolError::Validation(
                    "clipping_polygon must contain at least one polygon geometry".to_string(),
                ));
            }
        }
        let prepared_polys = Arc::new(prepared_polys);
        let use_clipping_polygon = !prepared_polys.is_empty();

        let mut dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;

        let col_center_x_vals: Vec<f64> = (0..cols).map(|c| dem.col_center_x(c)).collect();
        let row_center_y_vals: Vec<f64> = (0..rows).map(|r| dem.row_center_y(r)).collect();

        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        let apply_z_factor = (z_factor - 1.0).abs() > f32::EPSILON;
        for row in 0..rows {
            for col in 0..cols {
                let z = dem.get(0, row, col) as f32;
                if z == nodata_f32 {
                    continue;
                }
                let z_scaled = z * z_factor;
                min_z = min_z.min(z_scaled);
                max_z = max_z.max(z_scaled);
                if apply_z_factor {
                    dem.set(0, row, col, z_scaled as f64).map_err(|e| {
                        ToolError::Execution(format!("failed applying z_factor: {}", e))
                    })?;
                }
            }
        }
        if !min_z.is_finite() || !max_z.is_finite() {
            return Err(ToolError::Validation(
                "input DEM contains no valid cells".to_string(),
            ));
        }
        let range = (max_z - min_z).max(f32::EPSILON);

        let mut palette_vals = palette.get_palette();
        if reverse_palette {
            palette_vals.reverse();
        }
        let palette_vals: Vec<(f32, f32, f32)> = palette_vals
            .into_iter()
            .map(|(r, g, b)| (r.clamp(0.0, 255.0), g.clamp(0.0, 255.0), b.clamp(0.0, 255.0)))
            .collect();
        let p_last = palette_vals.len().saturating_sub(1) as f32;

        let mut cell_size_x = dem.cell_size_x.abs() as f32;
        let mut cell_size_y = dem.cell_size_y.abs() as f32;
        if Self::raster_is_geographic(&dem) {
            let center_lat = ((dem.y_min + dem.y_max()) * 0.5).to_radians() as f32;
            let meters_per_deg = 111_320.0_f32;
            let scale = center_lat.cos().abs().max(1.0e-8);
            cell_size_x *= meters_per_deg * scale;
            cell_size_y *= meters_per_deg;
        }

        let mut max_dist = Self::parse_max_dist(args, true);
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;
        let offsets = Arc::new(Self::compute_offsets(
            azimuth,
            max_dist,
            cell_size_x,
            cell_size_y,
            false,
        ));

        let sin_theta = altitude.to_radians().sin() as f64;
        let cos_theta = altitude.to_radians().cos() as f64;
        let shadow_light = 0.40_f64;
        let half_solar_diameter = 0.5_f64;
        let ambient_light = ambient_light.clamp(0.0, 1.0) as f64;
        let attenuation_parameter = attenuation_parameter.max(0.0) as f64;

        let width = cols as f32 * cell_size_x;
        let height = rows as f32 * cell_size_y;
        let radius = (width.max(height) * 0.5 * 2.0_f32.sqrt()).max(f32::EPSILON) as f64;
        let center_x = (cols as f64 - 1.0) * 0.5;
        let center_y = (rows as f64 - 1.0) * 0.5;
        let az_math = (90.0 - azimuth as f64).to_radians();
        let ls_rxy = radius * (altitude as f64).to_radians().cos();
        let ls_x = ls_rxy * az_math.cos();
        let ls_y = ls_rxy * az_math.sin();
        let ls_z = radius * (altitude as f64).to_radians().sin() + max_z as f64;

        let dx_by_col: Vec<f64> = (0..cols)
            .map(|col| (col as f64 - center_x) * cell_size_x as f64)
            .collect();
        let dy_by_row: Vec<f64> = (0..rows)
            .map(|row| (center_y - row as f64) * cell_size_y as f64)
            .collect();

        let clipping_mask = if use_clipping_polygon {
            let prepared_polys = prepared_polys.clone();
            let mask_rows: Vec<Vec<bool>> = (0..rows)
                .into_par_iter()
                .map(|row| {
                    let mut row_mask = vec![false; cols as usize];
                    let y = row_center_y_vals[row as usize];
                    for col in 0..cols {
                        let p = wbtopology::Coord::xy(col_center_x_vals[col as usize], y);
                        row_mask[col as usize] = !prepared_polys.iter().any(|poly| poly.contains_coord(p));
                    }
                    row_mask
                })
                .collect();
            Some(Arc::new(mask_rows))
        } else {
            None
        };

        const ATTENUATION_LUT_SIZE: usize = 4096;
        const ATTENUATION_LUT_MAX_DNORM: f64 = 4.0;
        let attenuation_lut = if attenuation_parameter > 0.0 {
            let mut lut = vec![0.0; ATTENUATION_LUT_SIZE + 1];
            for (i, val) in lut.iter_mut().enumerate() {
                let t = i as f64 / ATTENUATION_LUT_SIZE as f64;
                let d_norm = 1.0e-6 + t * (ATTENUATION_LUT_MAX_DNORM - 1.0e-6);
                *val = (1.0 / d_norm.powf(attenuation_parameter)).clamp(0.0, 1.0);
            }
            Some(Arc::new(lut))
        } else {
            None
        };

        let background_hgt = min_z - background_hgt_offset;
        let dem = Arc::new(dem);
        let resx = cell_size_x.max(f32::EPSILON) as f64;
        let resy = cell_size_y.max(f32::EPSILON) as f64;

        let row_data: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut out_row = vec![0.0; cols as usize];
                for col in 0..cols {
                    let mut zc = dem.get(0, row, col) as f32;
                    let clipped_out = clipping_mask
                        .as_ref()
                        .map(|m| m[row as usize][col as usize])
                        .unwrap_or(false);
                    let is_background = zc == nodata_f32 || clipped_out;
                    if is_background {
                        zc = background_hgt;
                    }

                    let mut r = background_clr[0] as f64;
                    let mut g = background_clr[1] as f64;
                    let mut b = background_clr[2] as f64;
                    let a = background_clr[3] as u32;

                    if !is_background {
                        let mut p = ((zc - min_z) / range).clamp(0.0, 1.0);
                        if !p.is_finite() {
                            p = 0.0;
                        }
                        let idxf = p * p_last;
                        let i0 = idxf.floor() as usize;
                        let i1 = (i0 + 1).min(palette_vals.len() - 1);
                        let t = (idxf - i0 as f32).clamp(0.0, 1.0);
                        let (r0, g0, b0) = palette_vals[i0];
                        let (r1, g1, b1) = palette_vals[i1];
                        r = (r0 + t * (r1 - r0)) as f64;
                        g = (g0 + t * (g1 - g0)) as f64;
                        b = (b0 + t * (b1 - b0)) as f64;
                    }

                    let z = |dr: isize, dc: isize| {
                        let v = dem.get(0, row + dr, col + dc) as f32;
                        if v == nodata_f32 {
                            zc
                        } else {
                            v
                        }
                    };
                    let z1 = z(-1, -1) as f64;
                    let z2 = z(-1, 0) as f64;
                    let z3 = z(-1, 1) as f64;
                    let z4 = z(0, -1) as f64;
                    let z6 = z(0, 1) as f64;
                    let z7 = z(1, -1) as f64;
                    let z8 = z(1, 0) as f64;
                    let z9 = z(1, 1) as f64;

                    let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * resx);
                    let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * resy);
                    let ts = (dzdx * dzdx + dzdy * dzdy).sqrt().max(0.00017);
                    let slope_term = ts / (1.0 + ts * ts).sqrt();

                    let mut aspect = if dzdx != 0.0 {
                        std::f64::consts::PI - (dzdy / dzdx).atan()
                            + (std::f64::consts::FRAC_PI_2 * (dzdx / dzdx.abs()))
                    } else {
                        std::f64::consts::PI
                    };
                    if !aspect.is_finite() {
                        aspect = std::f64::consts::PI;
                    }

                    let horizon_deg = if let Some((max_slope, _)) = SkyVisibilityCore::trace_horizon(
                        &dem,
                        row,
                        col,
                        &offsets,
                        nodata_f32,
                        0.0,
                        true,
                    ) {
                        max_slope.atan().to_degrees()
                    } else {
                        0.0
                    };

                    let mut shadow_factor = 1.0_f64;
                    let horizon_deg = horizon_deg as f64;
                    if horizon_deg >= altitude as f64 + half_solar_diameter {
                        shadow_factor = shadow_light;
                    } else if horizon_deg >= altitude as f64 - half_solar_diameter {
                        let frac = ((altitude as f64 + half_solar_diameter) - horizon_deg)
                            / (2.0 * half_solar_diameter);
                        shadow_factor = shadow_light + (1.0 - shadow_light) * frac.clamp(0.0, 1.0);
                    }

                    let term2 = sin_theta / ts;
                    let term3 = cos_theta
                        * (((azimuth as f64 - 90.0).to_radians()) - aspect).sin();
                    let hillshade = (slope_term * (term2 - term3)).clamp(0.0, 1.0);
                    let hillshade_factor = (hillshade + ambient_light).clamp(0.0, 1.0);

                    let dx = dx_by_col[col as usize];
                    let dy = dy_by_row[row as usize];
                    let d = ((ls_x - dx).powi(2) + (ls_y - dy).powi(2) + (ls_z - zc as f64).powi(2)).sqrt();
                    let d_norm = (d / radius).max(1.0e-6);
                    let attenuation = if attenuation_parameter <= 0.0 {
                        1.0
                    } else if let Some(lut) = attenuation_lut.as_ref() {
                        if d_norm >= ATTENUATION_LUT_MAX_DNORM {
                            (1.0 / d_norm.powf(attenuation_parameter)).clamp(0.0, 1.0)
                        } else {
                            let idx = ((d_norm / ATTENUATION_LUT_MAX_DNORM)
                                * ATTENUATION_LUT_SIZE as f64) as usize;
                            lut[idx]
                        }
                    } else {
                        (1.0 / d_norm.powf(attenuation_parameter)).clamp(0.0, 1.0)
                    };

                    let light = attenuation * shadow_factor * (0.5 + hillshade_factor * 0.5);
                    let r = (r * light).clamp(0.0, 255.0) as u32;
                    let g = (g * light).clamp(0.0, 255.0) as u32;
                    let b = (b * light).clamp(0.0, 255.0) as u32;
                    out_row[col as usize] = ((a << 24) | (b << 16) | (g << 8) | r) as f64;
                }
                out_row
            })
            .collect();

        let mut output = (*dem).clone();
        output.data_type = DataType::U32;
        output.nodata = 0.0;
        for (r, row) in row_data.iter().enumerate() {
            output
                .set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            ctx.progress.progress((r + 1) as f64 / rows as f64);
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn raster_is_geographic(input: &Raster) -> bool {
        let epsg = input.crs.epsg.or_else(|| {
            input
                .crs
                .wkt
                .as_deref()
                .and_then(|w| identify_epsg_from_wkt_with_policy(w, EpsgIdentifyPolicy::Lenient))
        });
        if let Some(code) = epsg {
            if let Ok(crs) = Crs::from_epsg(code) {
                return crs.is_geographic();
            }
        }
        false
    }

    fn infer_center_lat_lon(dem: &Raster) -> Result<(f64, f64), ToolError> {
        let center_x = (dem.x_min + dem.x_max()) * 0.5;
        let center_y = (dem.y_min + dem.y_max()) * 0.5;

        if Self::raster_is_geographic(dem) {
            return Ok((center_y.clamp(-90.0, 90.0), wbprojection::normalize_longitude(center_x)));
        }

        let src = if let Some(code) = dem.crs.epsg {
            Crs::from_epsg(code).map_err(|e| {
                ToolError::Validation(format!("failed parsing DEM CRS EPSG {}: {}", code, e))
            })?
        } else if let Some(wkt) = dem.crs.wkt.as_deref() {
            wbprojection::from_wkt(wkt).map_err(|e| {
                ToolError::Validation(format!("failed parsing DEM CRS WKT: {}", e))
            })?
        } else {
            return Err(ToolError::Validation(
                "unable to infer center latitude/longitude because DEM CRS metadata is missing; supply 'latitude' and 'longitude' explicitly"
                    .to_string(),
            ));
        };

        let wgs84 = Crs::from_epsg(4326)
            .map_err(|e| ToolError::Execution(format!("failed constructing EPSG:4326 CRS: {}", e)))?;
        let (lon, lat) = src.transform_to(center_x, center_y, &wgs84).map_err(|e| {
            ToolError::Execution(format!(
                "failed transforming DEM center to geographic coordinates: {}",
                e
            ))
        })?;

        if !lat.is_finite() || !lon.is_finite() {
            return Err(ToolError::Execution(
                "center coordinate transformation produced non-finite values".to_string(),
            ));
        }

        Ok((lat.clamp(-90.0, 90.0), wbprojection::normalize_longitude(lon)))
    }

    fn parse_utc_offset_hours(s: &str) -> Result<f64, ToolError> {
        let cleaned = s.trim().to_uppercase().replace("UTC", "");
        let negative = cleaned.starts_with('-');
        let cleaned = cleaned.replace('+', "").replace('-', "");
        let mut parts = cleaned.split(':');
        let hour_part = parts.next().unwrap_or("0").trim();
        let min_part = parts.next().unwrap_or("0").trim();

        let hours = hour_part.parse::<i32>().map_err(|_| {
            ToolError::Validation(format!("invalid utc_offset value '{}': bad hour component", s))
        })?;
        let minutes = min_part.parse::<i32>().map_err(|_| {
            ToolError::Validation(format!("invalid utc_offset value '{}': bad minute component", s))
        })?;

        if minutes < 0 || minutes >= 60 {
            return Err(ToolError::Validation(
                "utc_offset minutes must be in [0, 59]".to_string(),
            ));
        }

        let sign = if negative || hours < 0 { -1.0 } else { 1.0 };
        let offset = sign * (hours.abs() as f64 + (minutes as f64 / 60.0));
        if !(-12.0..=12.0).contains(&offset) {
            return Err(ToolError::Validation(
                "utc_offset must be between -12:00 and +12:00".to_string(),
            ));
        }
        Ok(offset)
    }

    fn parse_time_or_keyword(v: &str, sunrise_default: bool) -> Result<NaiveTime, ToolError> {
        let lower = v.trim().to_lowercase();
        if sunrise_default && lower.contains("sunrise") {
            return Ok(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        }
        if !sunrise_default && lower.contains("sunset") {
            return Ok(NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        }

        let parts: Vec<&str> = lower.split(':').collect();
        let h = parts
            .first()
            .ok_or_else(|| ToolError::Validation(format!("invalid time '{}'", v)))?
            .parse::<u32>()
            .map_err(|_| ToolError::Validation(format!("invalid time '{}'", v)))?;
        let m = if parts.len() > 1 {
            parts[1]
                .parse::<u32>()
                .map_err(|_| ToolError::Validation(format!("invalid time '{}'", v)))?
        } else {
            0
        };
        let s = if parts.len() > 2 {
            parts[2]
                .parse::<u32>()
                .map_err(|_| ToolError::Validation(format!("invalid time '{}'", v)))?
        } else {
            0
        };

        NaiveTime::from_hms_opt(h, m, s)
            .ok_or_else(|| ToolError::Validation(format!("invalid time '{}'", v)))
    }

    fn infer_utc_offset_hours_from_longitude(lon_deg: f64) -> f64 {
        // Approximate civil UTC offset from longitude when no explicit timezone is supplied.
        // Rounded to nearest half-hour to better represent common non-integer offsets.
        let raw = lon_deg / 15.0;
        let half_hour = (raw * 2.0).round() / 2.0;
        half_hour.clamp(-12.0, 12.0)
    }

    fn validate_time_in_daylight(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") && !args.contains_key("input") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        let az_fraction = args
            .get("az_fraction")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);
        if !(az_fraction > 0.0 && az_fraction < 360.0) {
            return Err(ToolError::Validation(
                "parameter 'az_fraction' must be in (0, 360)".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_average_horizon_distance(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        Ok(())
    }

    fn run_average_horizon_distance(
        args: &ToolArgs,
        _ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let az_fraction = args
            .get("az_fraction")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0) as f32;
        let mut max_dist = Self::parse_max_dist(args, true);
        let observer_hgt_offset = args
            .get("observer_hgt_offset")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05)
            .max(0.0) as f32;

        let dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;
        let cell_size_x = dem.cell_size_x as f32;
        let cell_size_y = dem.cell_size_y as f32;
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;

        let dem = Arc::new(dem);
        let mut sum = vec![0.0_f64; (rows * cols) as usize];
        let mut count = vec![0_u16; (rows * cols) as usize];

        let mut azimuth = 0.0_f32;
        while azimuth < 360.0 {
            let offsets = Arc::new(Self::compute_offsets(
                azimuth,
                max_dist,
                cell_size_x,
                cell_size_y,
                true,
            ));
            let num_threads = Self::num_threads();
            let (tx, rx) = mpsc::channel();

            for tid in 0..num_threads {
                let tx = tx.clone();
                let dem = dem.clone();
                let offsets = offsets.clone();
                thread::spawn(move || {
                    for row in (0..rows).filter(|r| r % num_threads == tid) {
                        let mut data = vec![nodata; cols as usize];
                        let mut n = vec![0_u8; cols as usize];
                        for col in 0..cols {
                            if let Some((max_slope, dist)) = SkyVisibilityCore::trace_horizon(
                                &dem,
                                row,
                                col,
                                &offsets,
                                nodata_f32,
                                observer_hgt_offset,
                                false,
                            ) {
                                if max_slope != 0.0 || dist != 0.0 {
                                    data[col as usize] = dist as f64;
                                    n[col as usize] = 1;
                                } else {
                                    data[col as usize] = 0.0;
                                    n[col as usize] = 1;
                                }
                            }
                        }
                        if tx.send((row, data, n)).is_err() {
                            return;
                        }
                    }
                });
            }
            drop(tx);

            for _ in 0..rows {
                let (row, data, n) = rx
                    .recv()
                    .map_err(|e| ToolError::Execution(format!("processing failed: {}", e)))?;
                for col in 0..cols {
                    let idx = (row * cols + col) as usize;
                    if data[col as usize] != nodata {
                        sum[idx] += data[col as usize];
                        count[idx] = count[idx].saturating_add(n[col as usize] as u16);
                    }
                }
            }

            azimuth += az_fraction;
        }

        let mut output = (*dem).clone();
        for row in 0..rows {
            let mut row_data = vec![nodata; cols as usize];
            for col in 0..cols {
                let idx = (row * cols + col) as usize;
                let z = dem.get(0, row, col) as f32;
                if z != nodata_f32 {
                    let n = count[idx] as f64;
                    row_data[col as usize] = if n > 0.0 { sum[idx] / n } else { 0.0 };
                }
            }
            output
                .set_row_slice(0, row, &row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn run_time_in_daylight(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let az_fraction = args
            .get("az_fraction")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0) as f32;
        if !(az_fraction > 0.0 && az_fraction < 360.0) {
            return Err(ToolError::Validation(
                "parameter 'az_fraction' must be in (0, 360)".to_string(),
            ));
        }

        let mut max_dist = Self::parse_max_dist(args, true);
        let utc_offset_arg = args
            .get("utc_offset")
            .or_else(|| args.get("utc_offset_str"))
            .and_then(|v| v.as_str());

        let start_day = args
            .get("start_day")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        let end_day = args
            .get("end_day")
            .and_then(|v| v.as_u64())
            .unwrap_or(365) as u32;
        if start_day < 1 || start_day > 366 || end_day < 1 || end_day > 366 || end_day < start_day {
            return Err(ToolError::Validation(
                "start_day/end_day must be in [1, 366] and start_day <= end_day".to_string(),
            ));
        }

        let start_time_text = args
            .get("start_time")
            .and_then(|v| v.as_str())
            .unwrap_or("sunrise");
        let end_time_text = args
            .get("end_time")
            .and_then(|v| v.as_str())
            .unwrap_or("sunset");
        let start_time = Self::parse_time_or_keyword(start_time_text, true)?;
        let end_time = Self::parse_time_or_keyword(end_time_text, false)?;
        if end_time < start_time {
            return Err(ToolError::Validation(
                "start_time must occur before end_time".to_string(),
            ));
        }

        let dem = Self::load_raster(&dem_path)?;
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let nodata_f32 = nodata as f32;

        let lat_opt = args.get("latitude").and_then(|v| v.as_f64());
        let lon_opt = args
            .get("longitude")
            .or_else(|| args.get("long"))
            .and_then(|v| v.as_f64());
        let (latitude, longitude) = match (lat_opt, lon_opt) {
            (Some(lat), Some(lon)) => (lat, lon),
            _ => Self::infer_center_lat_lon(&dem)?,
        };

        if !(-90.0..=90.0).contains(&latitude) {
            return Err(ToolError::Validation(
                "latitude must be in [-90, 90]".to_string(),
            ));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(ToolError::Validation(
                "longitude must be in [-180, 180]".to_string(),
            ));
        }

        let utc_offset = if let Some(s) = utc_offset_arg {
            Self::parse_utc_offset_hours(s)?
        } else {
            let inferred = Self::infer_utc_offset_hours_from_longitude(longitude);
            ctx.progress.info(&format!(
                "utc_offset not provided; inferred {:.1}h from longitude {:.6}",
                inferred, longitude
            ));
            inferred
        };

        let mut cell_size_x = dem.cell_size_x.abs() as f32;
        let mut cell_size_y = dem.cell_size_y.abs() as f32;
        if Self::raster_is_geographic(&dem) {
            let lat_rad = latitude.to_radians();
            let meters_per_deg = 111_320.0_f32;
            cell_size_x *= meters_per_deg * lat_rad.cos() as f32;
            cell_size_y *= meters_per_deg;
        }
        if cell_size_x <= 0.0 || cell_size_y <= 0.0 {
            return Err(ToolError::Validation(
                "invalid DEM cell size; expected positive resolution".to_string(),
            ));
        }

        let diag_m = ((rows as f32 * cell_size_y).powi(2) + (cols as f32 * cell_size_x).powi(2)).sqrt();
        if max_dist.is_infinite() {
            max_dist = diag_m;
        } else {
            if max_dist <= 5.0 * cell_size_x {
                return Err(ToolError::Validation(
                    "max_dist must be larger than 5x cell size".to_string(),
                ));
            }
            max_dist = max_dist.min(diag_m);
        }

        let almanac = generate_almanac(latitude, longitude, utc_offset, az_fraction as f64, 10)?;
        let num_bins = ((360.0_f64 / az_fraction as f64).ceil() as usize).max(1);

        let dem = Arc::new(dem);
        let mut shadow_seconds = vec![0.0_f64; (rows * cols) as usize];
        let mut total_daylight = 0.0_f64;

        for bin in 0..num_bins {
            let azimuth = bin as f32 * az_fraction;
            let mut entries: Vec<(f32, f64, NaiveTime, u32)> = Vec::with_capacity(almanac.len());
            let mut daylight_in_bin = 0.0_f64;

            for day in &almanac {
                let sample = day.bins[bin];
                entries.push((sample.altitude, sample.duration, sample.time, day.ordinal));
                if day.ordinal >= start_day
                    && day.ordinal <= end_day
                    && sample.time >= start_time
                    && sample.time <= end_time
                    && sample.duration > 0.0
                {
                    daylight_in_bin += sample.duration;
                }
            }
            total_daylight += daylight_in_bin;

            entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
            if entries.first().map(|e| e.1).unwrap_or(0.0) <= 0.0 || daylight_in_bin <= 0.0 {
                ctx.progress.progress((bin + 1) as f64 / num_bins as f64);
                continue;
            }

            let offsets = Arc::new(Self::compute_offsets(azimuth, max_dist, cell_size_x, cell_size_y, false));
            let num_threads = Self::num_threads();
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_threads {
                let tx = tx.clone();
                let dem = dem.clone();
                let offsets = offsets.clone();
                thread::spawn(move || {
                    for row in (0..rows).filter(|r| r % num_threads == tid) {
                        let mut ha_row = vec![nodata_f32; cols as usize];
                        for col in 0..cols {
                            if let Some((max_slope, _)) = SkyVisibilityCore::trace_horizon(
                                &dem,
                                row,
                                col,
                                &offsets,
                                nodata_f32,
                                0.0,
                                true,
                            ) {
                                ha_row[col as usize] = max_slope.atan().to_degrees();
                            }
                        }
                        if tx.send((row, ha_row)).is_err() {
                            return;
                        }
                    }
                });
            }
            drop(tx);

            for row_i in 0..rows {
                let (row, ha_row) = rx
                    .recv()
                    .map_err(|e| ToolError::Execution(format!("processing failed: {}", e)))?;
                let shadow_row: Vec<f64> = ha_row
                    .par_iter()
                    .map(|ha| {
                        if *ha == nodata_f32 {
                            return f64::NAN;
                        }
                        let mut sec_shadow = 0.0_f64;
                        for (alt, dur, t, ord) in &entries {
                            if *dur <= 0.0 {
                                break;
                            }
                            if *ord < start_day || *ord > end_day || *t < start_time || *t > end_time {
                                continue;
                            }
                            if *alt < *ha {
                                sec_shadow += *dur;
                            }
                        }
                        sec_shadow
                    })
                    .collect();

                for col in 0..cols {
                    let idx = (row * cols + col) as usize;
                    let v = shadow_row[col as usize];
                    if v.is_finite() {
                        shadow_seconds[idx] += v;
                    }
                }

                let p = (bin as f64 + (row_i + 1) as f64 / rows as f64) / num_bins as f64;
                ctx.progress.progress(p);
            }
        }

        let mut output = (*dem).clone();
        for row in 0..rows {
            let mut row_data = vec![nodata; cols as usize];
            for col in 0..cols {
                let z = dem.get(0, row, col) as f32;
                if z != nodata_f32 {
                    let idx = (row * cols + col) as usize;
                    row_data[col as usize] = if total_daylight > 0.0 {
                        1.0 - shadow_seconds[idx] / total_daylight
                    } else {
                        0.0
                    };
                }
            }
            output
                .set_row_slice(0, row, &row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        let out = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(out))
    }

    fn skyline_analysis_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "skyline_analysis",
            display_name: "Skyline Analysis",
            summary: "Performs skyline analysis for one or more observation points and writes a vector horizon trace plus HTML report.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM/DSM raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "points",
                    description: "Input point vector path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "az_fraction",
                    description: "Azimuth step in degrees [0.01, 45].",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_dist",
                    description: "Maximum search distance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "observer_hgt_offset",
                    description: "Observer height offset above terrain.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_as_polygons",
                    description: "Write polygon horizons instead of polylines.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output_html",
                    description: "Optional HTML report path.",
                    required: false,
                },
            ],
        }
    }

    fn skyline_analysis_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dsm.tif"));
        defaults.insert("points".to_string(), json!("stations.shp"));
        defaults.insert("az_fraction".to_string(), json!(1.0));
        defaults.insert("max_dist".to_string(), json!("inf"));
        defaults.insert("observer_hgt_offset".to_string(), json!(0.05));
        defaults.insert("output_as_polygons".to_string(), json!(true));
        defaults.insert("output".to_string(), json!("skyline.shp"));
        defaults.insert("output_html".to_string(), json!("skyline.html"));

        ToolManifest {
            id: "skyline_analysis".to_string(),
            display_name: "Skyline Analysis".to_string(),
            summary: "Performs skyline analysis for one or more observation points and writes a vector horizon trace plus HTML report."
                .to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "visibility".to_string(),
                "skyline".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate_skyline_analysis(args: &ToolArgs) -> Result<(), ToolError> {
        if !args.contains_key("dem") && !args.contains_key("input") {
            return Err(ToolError::Validation(
                "missing required parameter 'dem'".to_string(),
            ));
        }
        parse_vector_path_arg(args, "points")?;
        let _ = parse_optional_output_path(args, "output")?
            .ok_or_else(|| ToolError::Validation("missing required parameter 'output'".to_string()))?;
        if let Some(v) = args.get("az_fraction").and_then(|v| v.as_f64()) {
            if !(0.01..=45.0).contains(&v) {
                return Err(ToolError::Validation(
                    "parameter 'az_fraction' must be in [0.01, 45]".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn run_skyline_analysis(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_dem_input(args)?;
        let points_path = parse_vector_path_arg(args, "points")?;
        let output_path = parse_optional_output_path(args, "output")?
            .ok_or_else(|| ToolError::Validation("missing required parameter 'output'".to_string()))?;
        let output_html = parse_optional_output_path(args, "output_html")?
            .unwrap_or_else(|| output_path.with_extension("html"));
        let az_fraction = args
            .get("az_fraction")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let mut max_dist = Self::parse_max_dist(args, true);
        let observer_hgt_offset = args
            .get("observer_hgt_offset")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05)
            .max(0.0) as f32;
        let output_as_polygons = args
            .get("output_as_polygons")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if !(0.01..=45.0).contains(&az_fraction) {
            return Err(ToolError::Validation(
                "parameter 'az_fraction' must be in [0.01, 45]".to_string(),
            ));
        }

        let dem = Self::load_raster(&dem_path)?;
        let nodata_f32 = dem.nodata as f32;
        let cell_size_x = dem.cell_size_x as f32;
        let cell_size_y = dem.cell_size_y as f32;
        max_dist = Self::clamp_max_dist(max_dist, &dem, cell_size_x)?;

        let stations = Self::parse_station_points(&points_path)?;

        let mut out_layer = Layer::new("skyline").with_geom_type(if output_as_polygons {
            GeometryType::Polygon
        } else {
            GeometryType::LineString
        });
        if let Some(epsg) = dem.crs.epsg {
            out_layer = out_layer.with_epsg(epsg);
        }
        if let Some(wkt) = dem.crs.wkt.as_ref() {
            out_layer = out_layer.with_crs_wkt(wkt.clone());
        }
        out_layer.add_field(FieldDef::new("FID", FieldType::Integer));
        out_layer.add_field(FieldDef::new("AVG_ZENITH", FieldType::Float));
        out_layer.add_field(FieldDef::new("AVG_DIST", FieldType::Float));
        out_layer.add_field(FieldDef::new("HORZN_AREA", FieldType::Float));
        out_layer.add_field(FieldDef::new("SKYLN_ELEV", FieldType::Float));
        out_layer.add_field(FieldDef::new("STDEV_ELEV", FieldType::Float));
        out_layer.add_field(FieldDef::new("SVF", FieldType::Float));

        let mut html = String::from(
            "<!doctype html><html><head><meta charset='utf-8'><title>Skyline Analysis</title><style>body{margin:0;background:#f5f5f2;font-family:Helvetica,Arial,sans-serif;color:#111}main{max-width:1280px;margin:0 auto;padding:24px}h1{margin:0 0 16px 0}.meta{margin:0 0 18px 0;color:#333}.card{background:#fff;border:1px solid #ddd;border-radius:14px;padding:18px;margin:0 0 18px 0;box-shadow:0 10px 24px rgba(0,0,0,.05)}table{border-collapse:collapse;width:100%;max-width:520px}th,td{text-align:left;padding:6px 8px;border-bottom:1px solid #eee}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:16px}</style></head><body><main><h1>Skyline Analysis</h1>",
        );
        html.push_str(&format!(
            "<p class='meta'><strong>Input DEM:</strong> {}<br><strong>Input points:</strong> {}<br><strong>Observer height offset:</strong> {:.3}<br><strong>Maximum distance:</strong> {:.3}</p>",
            dem_path,
            points_path,
            observer_hgt_offset,
            max_dist,
        ));

        let mut fid = 1_i64;
        for (si, station) in stations.iter().enumerate() {
            let Some((col, row)) = dem.world_to_pixel(station.x, station.y) else {
                continue;
            };
            let base_elev = dem.get(0, row, col) as f32;
            if !base_elev.is_finite() || base_elev == nodata_f32 {
                continue;
            }

            let mut skyline_points = Vec::<VCoord>::new();
            let mut zenith_values = Vec::<f64>::new();
            let mut horizon_distances = Vec::<f64>::new();
            let mut skyline_elevations = Vec::<f64>::new();

            let mut azimuth = 0.0_f32;
            while azimuth < 360.0 {
                let offsets = Self::compute_offsets(
                    azimuth,
                    max_dist,
                    cell_size_x,
                    cell_size_y,
                    true,
                );
                let (max_slope, dist) = Self::trace_horizon(
                    &dem,
                    row,
                    col,
                    &offsets,
                    nodata_f32,
                    observer_hgt_offset,
                    false,
                )
                .unwrap_or((0.0, 0.0));

                let horizon_angle_deg = max_slope.atan().to_degrees() as f64;
                let zenith_deg = 90.0 - horizon_angle_deg;
                let azimuth_rad = (azimuth as f64).to_radians();
                let x = station.x + dist as f64 * azimuth_rad.cos();
                let y = station.y + dist as f64 * azimuth_rad.sin();
                let z = base_elev as f64 + observer_hgt_offset as f64 + max_slope as f64 * dist as f64;

                skyline_points.push(VCoord {
                    x,
                    y,
                    z: Some(z),
                    m: Some(zenith_deg),
                });
                zenith_values.push(zenith_deg);
                horizon_distances.push(dist as f64);
                skyline_elevations.push(z);
                azimuth += az_fraction;
            }

            if skyline_points.is_empty() {
                continue;
            }

            let n = zenith_values.len() as f64;
            let avg_zenith = zenith_values.iter().sum::<f64>() / n;
            let avg_horizon_distance = horizon_distances.iter().sum::<f64>() / n;
            let avg_elevation = skyline_elevations.iter().sum::<f64>() / n;
            let stdev_elevation = (skyline_elevations
                .iter()
                .map(|e| (e - avg_elevation).powi(2))
                .sum::<f64>()
                / n)
                .sqrt();

            let mut svf_acc = 0.0;
            for z in &zenith_values {
                let ha = (90.0 - *z).to_radians();
                svf_acc += ha.sin().max(0.0);
            }
            let svf = (1.0 - (svf_acc / n)).clamp(0.0, 1.0);

            let mut area_points: Vec<(f64, f64)> = skyline_points.iter().map(|c| (c.x, c.y)).collect();
            if area_points.first() != area_points.last() {
                area_points.push(area_points[0]);
            }
            let mut horizon_area = 0.0;
            for i in 0..(area_points.len().saturating_sub(1)) {
                horizon_area += area_points[i].0 * area_points[i + 1].1
                    - area_points[i + 1].0 * area_points[i].1;
            }
            horizon_area = (horizon_area * 0.5).abs();

            let geom = if output_as_polygons {
                Geometry::Polygon {
                    exterior: wbvector::Ring::new(skyline_points.clone()),
                    interiors: vec![],
                }
            } else {
                Geometry::LineString(skyline_points.clone())
            };

            out_layer
                .add_feature(
                    Some(geom),
                    &[
                        ("FID", FieldValue::Integer(fid)),
                        ("AVG_ZENITH", FieldValue::Float(avg_zenith)),
                        ("AVG_DIST", FieldValue::Float(avg_horizon_distance)),
                        ("HORZN_AREA", FieldValue::Float(horizon_area)),
                        ("SKYLN_ELEV", FieldValue::Float(avg_elevation)),
                        ("STDEV_ELEV", FieldValue::Float(stdev_elevation)),
                        ("SVF", FieldValue::Float(svf)),
                    ],
                )
                .map_err(|e| ToolError::Execution(format!("failed adding skyline feature: {}", e)))?;

            html.push_str(&format!(
                "<div class='card'><h3 style='margin:0 0 8px 0'>Site {}</h3><table><tr><th>Metric</th><th>Value</th></tr><tr><td>Avg. zenith angle</td><td>{:.2} deg</td></tr><tr><td>Avg. horizon distance</td><td>{:.2}</td></tr><tr><td>Horizon area</td><td>{:.3}</td></tr><tr><td>Avg. skyline elevation</td><td>{:.2}</td></tr><tr><td>Std. dev. skyline elevation</td><td>{:.2}</td></tr><tr><td>Sky-view factor</td><td>{:.2}</td></tr></table><p style='margin:10px 0 6px 0'><strong>Zenith angle and horizon distance plots:</strong></p><div class='grid'><div>{}</div><div>{}</div></div></div>",
                si + 1,
                avg_zenith,
                avg_horizon_distance,
                horizon_area,
                avg_elevation,
                stdev_elevation,
                svf,
                Self::radial_svg(560.0, 420.0, &zenith_values, "Zenith angle", "#2977c9"),
                Self::radial_svg(560.0, 420.0, &horizon_distances, "Horizon distance", "#c23b22")
            ));

            fid += 1;
            ctx.progress.progress((si + 1) as f64 / stations.len() as f64);
        }

        if out_layer.features.is_empty() {
            return Err(ToolError::Validation(
                "skyline_analysis did not produce any output features; check point locations against DEM extent"
                    .to_string(),
            ));
        }

        html.push_str("</main></body></html>");
        if let Some(parent) = output_html.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("Failed to create output directory: {}", e))
                })?;
            }
        }
        std::fs::write(&output_html, html)
            .map_err(|e| ToolError::Execution(format!("failed writing HTML report: {}", e)))?;

        let output_path_str = output_path.to_string_lossy().to_string();
        Self::write_vector_output(&out_layer, &output_path_str)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_path_str));
        outputs.insert(
            "report_path".to_string(),
            json!(output_html.to_string_lossy().to_string()),
        );
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }
}

#[derive(Clone, Copy)]
struct AlmanacSample {
    altitude: f32,
    time: NaiveTime,
    diff: f64,
    duration: f64,
}

impl Default for AlmanacSample {
    fn default() -> Self {
        Self {
            altitude: 0.0,
            time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            diff: 360.0,
            duration: 0.0,
        }
    }
}

#[derive(Clone)]
struct AlmanacDay {
    ordinal: u32,
    bins: Vec<AlmanacSample>,
}

fn generate_almanac(
    latitude: f64,
    longitude: f64,
    utc_offset_hours: f64,
    az_interval: f64,
    seconds_interval: usize,
) -> Result<Vec<AlmanacDay>, ToolError> {
    let tz_secs = (utc_offset_hours * 3600.0).round() as i32;
    let tz = if tz_secs < 0 {
        FixedOffset::west_opt(-tz_secs)
    } else {
        FixedOffset::east_opt(tz_secs)
    }
    .ok_or_else(|| ToolError::Validation("invalid utc_offset".to_string()))?;

    let num_bins = (360.0_f64 / az_interval).ceil() as usize;
    let mut out: Vec<AlmanacDay> = Vec::with_capacity(366);

    for doy in 1..=366_u32 {
        let date = NaiveDate::from_yo_opt(2020, doy).ok_or_else(|| {
            ToolError::Execution(format!("failed constructing date for day {}", doy))
        })?;
        let mut day = AlmanacDay {
            ordinal: date.ordinal(),
            bins: vec![AlmanacSample::default(); num_bins],
        };

        for hr in 0..24 {
            for minute in 0..60 {
                for sec in (0..=45).step_by(seconds_interval) {
                    let dt = tz
                        .from_local_datetime(&date.and_hms_opt(hr, minute, sec as u32).ok_or_else(|| {
                            ToolError::Execution("failed constructing datetime".to_string())
                        })?)
                        .single()
                        .ok_or_else(|| ToolError::Execution("invalid local datetime".to_string()))?;

                    let unix_ms = dt.timestamp_millis();
                    let pos = solar_pos(unix_ms, latitude, longitude);
                    let az = pos.azimuth.to_degrees();
                    let alt = pos.altitude.to_degrees();

                    let mut bin = (az / az_interval).round() as usize;
                    let mut bin_az = bin as f64 * az_interval;
                    if bin == num_bins || (bin_az - 360.0).abs() <= f64::EPSILON {
                        bin = 0;
                        bin_az = 0.0;
                    }

                    let diff = (bin_az - az).abs();
                    if diff < day.bins[bin].diff {
                        day.bins[bin].diff = diff;
                        day.bins[bin].altitude = alt as f32;
                        day.bins[bin].time = dt.time();
                    }
                    if alt >= -0.5 {
                        day.bins[bin].duration += seconds_interval as f64;
                    }
                }
            }
        }

        out.push(day);
    }

    Ok(out)
}

#[derive(Clone, Copy)]
struct SolarPosition {
    azimuth: f64,
    altitude: f64,
}

const MILLISECONDS_PER_DAY: f64 = 86_400_000.0;
const J1970: f64 = 2_440_588.0;
const J2000: f64 = 2_451_545.0;
const TO_RAD: f64 = std::f64::consts::PI / 180.0;
const OBLIQUITY_OF_EARTH: f64 = 23.4397 * TO_RAD;
const PERIHELION_OF_EARTH: f64 = 102.9372 * TO_RAD;

#[inline]
fn to_julian(unixtime_ms: i64) -> f64 {
    unixtime_ms as f64 / MILLISECONDS_PER_DAY - 0.5 + J1970
}

#[inline]
fn to_days(unixtime_ms: i64) -> f64 {
    to_julian(unixtime_ms) - J2000
}

#[inline]
fn right_ascension(l: f64, b: f64) -> f64 {
    (l.sin() * OBLIQUITY_OF_EARTH.cos() - b.tan() * OBLIQUITY_OF_EARTH.sin()).atan2(l.cos())
}

#[inline]
fn declination(l: f64, b: f64) -> f64 {
    (b.sin() * OBLIQUITY_OF_EARTH.cos() + b.cos() * OBLIQUITY_OF_EARTH.sin() * l.sin()).asin()
}

#[inline]
fn solar_azimuth(h: f64, phi: f64, dec: f64) -> f64 {
    h.sin().atan2(h.cos() * phi.sin() - dec.tan() * phi.cos()) + std::f64::consts::PI
}

#[inline]
fn solar_altitude(h: f64, phi: f64, dec: f64) -> f64 {
    (phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos()).asin()
}

#[inline]
fn sidereal_time(d: f64, lw: f64) -> f64 {
    (280.16 + 360.985_623_5 * d).to_radians() - lw
}

#[inline]
fn solar_mean_anomaly(d: f64) -> f64 {
    (357.5291 + 0.985_600_28 * d).to_radians()
}

#[inline]
fn equation_of_center(m: f64) -> f64 {
    (1.9148 * m.sin() + 0.02 * (2.0 * m).sin() + 0.0003 * (3.0 * m).sin()).to_radians()
}

#[inline]
fn ecliptic_longitude(m: f64) -> f64 {
    m + equation_of_center(m) + PERIHELION_OF_EARTH + std::f64::consts::PI
}

fn solar_pos(unixtime_ms: i64, lat_deg: f64, lon_deg: f64) -> SolarPosition {
    let lw = -lon_deg.to_radians();
    let phi = lat_deg.to_radians();
    let d = to_days(unixtime_ms);
    let m = solar_mean_anomaly(d);
    let l = ecliptic_longitude(m);
    let dec = declination(l, 0.0);
    let ra = right_ascension(l, 0.0);
    let h = sidereal_time(d, lw) - ra;
    SolarPosition {
        azimuth: solar_azimuth(h, phi, dec),
        altitude: solar_altitude(h, phi, dec),
    }
}

impl Tool for HorizonAngleTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::horizon_angle_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::horizon_angle_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_horizon_angle(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_horizon_angle(args, ctx)
    }
}

impl Tool for SkyViewFactorTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::sky_view_factor_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::sky_view_factor_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_sky_view_factor(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_sky_view_factor(args, ctx)
    }
}

impl Tool for VisibilityIndexTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::visibility_index_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::visibility_index_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_visibility_index(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_visibility_index(args, ctx)
    }
}

impl Tool for HorizonAreaTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::horizon_area_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::horizon_area_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_horizon_area(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_horizon_area(args, ctx)
    }
}

impl Tool for AverageHorizonDistanceTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::average_horizon_distance_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::average_horizon_distance_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_average_horizon_distance(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_average_horizon_distance(args, ctx)
    }
}

impl Tool for TimeInDaylightTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::time_in_daylight_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::time_in_daylight_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_time_in_daylight(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_time_in_daylight(args, ctx)
    }
}

impl Tool for ShadowImageTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::shadow_image_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::shadow_image_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_shadow_image(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_shadow_image(args, ctx)
    }
}

impl Tool for ShadowAnimationTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::shadow_animation_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::shadow_animation_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_shadow_animation(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_shadow_animation(args, ctx)
    }
}

impl Tool for HypsometricallyTintedHillshadeTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::hypsometrically_tinted_hillshade_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::hypsometrically_tinted_hillshade_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_hypsometrically_tinted_hillshade(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_hypsometrically_tinted_hillshade(args, ctx)
    }
}

impl Tool for TopoRenderTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::topo_render_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::topo_render_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_topo_render(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_topo_render(args, ctx)
    }
}

impl Tool for SkylineAnalysisTool {
    fn metadata(&self) -> ToolMetadata {
        SkyVisibilityCore::skyline_analysis_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        SkyVisibilityCore::skyline_analysis_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        SkyVisibilityCore::validate_skyline_analysis(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        SkyVisibilityCore::run_skyline_analysis(args, ctx)
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

    fn make_ramp_raster(rows: usize, cols: usize) -> Raster {
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
                raster
                    .set(0, row, col, row as f64 * 2.0 + col as f64)
                    .unwrap();
            }
        }
        raster
    }

    #[test]
    fn shadow_image_flat_surface_produces_valid_intensity() {
        let dem = make_constant_raster(7, 7, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("max_dist".to_string(), json!(100.0));
        args.insert("palette".to_string(), json!("white"));
        args.insert("date".to_string(), json!("21/06/2021"));
        args.insert("time".to_string(), json!("13:00"));
        args.insert("location".to_string(), json!("43.5448/-80.2482/-4"));

        let result = ShadowImageTool.run(&args, &make_ctx()).unwrap();
        let out_id =
            memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap())
                .unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let v = out.get(0, 3, 3);
        assert!(v.is_finite());
        assert!(v >= 0.0 && v <= 1.0);
    }

    #[test]
    fn shadow_image_palette_output_is_u32_color() {
        let dem = make_constant_raster(7, 7, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("max_dist".to_string(), json!(100.0));
        args.insert("palette".to_string(), json!("soft"));
        args.insert("date".to_string(), json!("21/06/2021"));
        args.insert("time".to_string(), json!("13:00"));
        args.insert("location".to_string(), json!("43.5448/-80.2482/-4"));

        let result = ShadowImageTool.run(&args, &make_ctx()).unwrap();
        let out_id =
            memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap())
                .unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert_eq!(out.data_type, DataType::U32);
        let v = out.get(0, 3, 3);
        assert!(v.is_finite());
        assert!(v >= 0.0);
    }

    #[test]
    fn topo_render_runs_and_returns_rgb() {
        let dem = make_constant_raster(9, 9, 25.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("palette".to_string(), json!("soft"));
        args.insert("azimuth".to_string(), json!(315.0));
        args.insert("altitude".to_string(), json!(30.0));

        let result = TopoRenderTool.run(&args, &make_ctx()).unwrap();
        let out_id =
            memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap())
                .unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert_eq!(out.data_type, DataType::U32);
        let v = out.get(0, 4, 4);
        assert!(v.is_finite());
        assert!(v > 0.0);
    }

    #[test]
    fn hypsometrically_tinted_hillshade_runs_and_returns_rgb() {
        let dem = make_ramp_raster(9, 9);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("palette".to_string(), json!("atlas"));
        args.insert("solar_altitude".to_string(), json!(45.0));
        args.insert("hillshade_weight".to_string(), json!(0.5));

        let result = HypsometricallyTintedHillshadeTool
            .run(&args, &make_ctx())
            .unwrap();
        let out_id =
            memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap())
                .unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert_eq!(out.data_type, DataType::U32);
        let v = out.get(0, 4, 4);
        assert!(v.is_finite());
        assert!(v > 0.0);
    }

    #[test]
    fn hypsometrically_tinted_hillshade_accepts_legacy_alias_params() {
        let dem = make_ramp_raster(11, 11);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("palette".to_string(), json!("atlas"));
        args.insert("altitude".to_string(), json!(50.0));
        args.insert("hs_weight".to_string(), json!(0.6));
        args.insert("atmospheric".to_string(), json!(0.2));

        let result = HypsometricallyTintedHillshadeTool
            .run(&args, &make_ctx())
            .unwrap();
        let out_id =
            memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap())
                .unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert_eq!(out.data_type, DataType::U32);
        let v = out.get(0, 5, 5);
        assert!(v.is_finite());
        assert!(v > 0.0);
    }

    #[test]
    fn hypsometrically_tinted_hillshade_full_360_and_atmospheric_extreme_are_valid() {
        let dem = make_ramp_raster(11, 11);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("palette".to_string(), json!("atlas"));
        args.insert("full_360_mode".to_string(), json!(true));
        args.insert("atmospheric_effects".to_string(), json!(1.0));
        args.insert("brightness".to_string(), json!(0.7));

        let result = HypsometricallyTintedHillshadeTool
            .run(&args, &make_ctx())
            .unwrap();
        let out_id =
            memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap())
                .unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert_eq!(out.data_type, DataType::U32);
        let center = out.get(0, 5, 5);
        let corner = out.get(0, 1, 1);
        assert!(center.is_finite() && center > 0.0);
        assert!(corner.is_finite() && corner > 0.0);
    }

    #[test]
    fn skyline_analysis_writes_vector_and_html() {
        let dem = make_constant_raster(31, 31, 100.0);
        let dem_id = memory_store::put_raster(dem.clone());

        let tmp_dir = std::env::temp_dir().join("wbtools_oss_skyline_analysis_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let points_path = tmp_dir.join("stations.shp");
        let output_vec = tmp_dir.join("skyline.shp");
        let output_html = tmp_dir.join("skyline.html");

        let mut pts = Layer::new("stations").with_geom_type(GeometryType::Point);
        let x = dem.col_center_x(15);
        let y = dem.row_center_y(15);
        pts.add_feature(
            Some(Geometry::Point(VCoord {
                x,
                y,
                z: None,
                m: None,
            })),
            &[],
        )
        .unwrap();
        wbvector::write(&pts, points_path.to_string_lossy().as_ref(), VectorFormat::Shapefile).unwrap();

        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&dem_id)),
        );
        args.insert(
            "points".to_string(),
            json!(points_path.to_string_lossy().to_string()),
        );
        args.insert("az_fraction".to_string(), json!(10.0));
        args.insert(
            "output".to_string(),
            json!(output_vec.to_string_lossy().to_string()),
        );
        args.insert(
            "output_html".to_string(),
            json!(output_html.to_string_lossy().to_string()),
        );

        let result = SkylineAnalysisTool.run(&args, &make_ctx()).unwrap();
        let path = result.outputs.get("path").and_then(|v| v.as_str()).unwrap();
        let report = result
            .outputs
            .get("report_path")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(std::path::Path::new(path).exists());
        assert!(std::path::Path::new(report).exists());
    }

    #[test]
    fn shadow_animation_writes_html_and_gif() {
        let dem = make_constant_raster(7, 7, 100.0);
        let id = memory_store::put_raster(dem);

        let tmp_dir = std::env::temp_dir().join("wbtools_oss_shadow_animation_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let output_html = tmp_dir.join("shadow_animation.html");
        let output_gif = tmp_dir.join("shadow_animation.gif");

        let mut args = ToolArgs::new();
        args.insert("dem".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("date".to_string(), json!("21/06/2021"));
        args.insert("time_interval".to_string(), json!(60u64));
        args.insert("location".to_string(), json!("43.5448/-80.2482/-4"));
        args.insert("palette".to_string(), json!("soft"));
        args.insert("image_height".to_string(), json!(50u64));
        args.insert("delay".to_string(), json!(250u64));
        args.insert("output".to_string(), json!(output_html.to_string_lossy().to_string()));

        let result = ShadowAnimationTool.run(&args, &make_ctx()).unwrap();
        let html_path = result.outputs.get("path").and_then(|v| v.as_str()).unwrap();
        let gif_path = result.outputs.get("gif_path").and_then(|v| v.as_str()).unwrap();
        assert!(std::path::Path::new(html_path).exists(), "HTML file not found: {html_path}");
        assert!(std::path::Path::new(gif_path).exists(), "GIF file not found: {gif_path}");
        assert_eq!(std::path::Path::new(gif_path), output_gif);
    }
}