use std::collections::BTreeMap;
use std::sync::Arc;

use rayon::prelude::*;
use serde_json::json;
use wbprojection::{identify_epsg_from_wkt_with_policy, Crs, EpsgIdentifyPolicy};
use wbcore::{PercentCoalescer, 
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterConfig, RasterFormat};

use crate::memory_store;

pub struct SlopeTool;
pub struct AspectTool;
pub struct ConvergenceIndexTool;
pub struct HillshadeTool;
pub struct MultidirectionalHillshadeTool;

/// Compute slope (degrees) and aspect (degrees clockwise from north) in one pass.
///
/// This is a workflow-oriented helper that avoids running separate tool dispatches
/// when both derivatives are needed together.
pub fn slope_aspect_from_dem(input: &Raster, z_factor: f64) -> Result<(Raster, Raster), ToolError> {
    let mut slope = Raster::new_like(input);
    let mut aspect = Raster::new_like(input);
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let dx = input.cell_size_x.abs().max(f64::EPSILON);
    let dy = input.cell_size_y.abs().max(f64::EPSILON);
    let is_geographic = TerrainCore::raster_is_geographic(input);
    let n = input.rows * input.cols * input.bands;
    let band_stride = rows * cols;

    let values: Vec<(f64, f64)> = (0..n)
        .into_par_iter()
        .map(|i| {
            let band = (i / band_stride) as isize;
            let rc = i % band_stride;
            let row = (rc / cols) as isize;
            let col = (rc % cols) as isize;

            let Some((p, q)) = (if is_geographic {
                TerrainCore::pq_geographic(input, band, row, col, z_factor)
            } else {
                TerrainCore::pq_projected(input, band, row, col, z_factor, dx, dy)
            }) else {
                return (nodata, nodata);
            };

            let t = p.mul_add(p, q * q).sqrt();
            let slope_v = t.atan().to_degrees();
            let aspect_v = if t <= 0.0 {
                -1.0
            } else {
                let mut a = 180.0 - (q / p).atan().to_degrees() + 90.0 * p.signum();
                if a >= 360.0 {
                    a -= 360.0;
                }
                a
            };
            (slope_v, aspect_v)
        })
        .collect();

    for (i, (s, a)) in values.into_iter().enumerate() {
        slope.data.set_f64(i, s);
        aspect.data.set_f64(i, a);
    }

    Ok((slope, aspect))
}

struct TerrainCore;

impl TerrainCore {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn parse_z_factor(args: &ToolArgs) -> f64 {
        args.get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
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

    #[inline]
    fn haversine_distance_m(lat1_deg: f64, lon1_deg: f64, lat2_deg: f64, lon2_deg: f64) -> f64 {
        let r = 6_371_008.8_f64;
        let lat1 = lat1_deg.to_radians();
        let lon1 = lon1_deg.to_radians();
        let lat2 = lat2_deg.to_radians();
        let lon2 = lon2_deg.to_radians();
        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;
        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        r * c
    }

    fn neighbourhood(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
    ) -> Option<[f64; 9]> {
        let z5 = input.get(band, row, col);
        if input.is_nodata(z5) {
            return None;
        }
        let offsets = [
            (-1isize, -1isize),
            (0, -1),
            (1, -1),
            (-1, 0),
            (0, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        let mut z = [0.0f64; 9];
        for (i, (ox, oy)) in offsets.iter().enumerate() {
            let v = input.get(band, row + *oy, col + *ox);
            z[i] = if input.is_nodata(v) {
                z5 * z_factor
            } else {
                v * z_factor
            };
        }
        Some(z)
    }

    /// Get a 5x5 neighbourhood for Florinsky-based gradient calculations (projected coords).
    #[inline(always)]
    fn neighbourhood_5x5(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
    ) -> Option<[f64; 25]> {
        let zcenter = input.get(band, row, col);
        if input.is_nodata(zcenter) {
            return None;
        }
        let nodata = input.nodata;
        let zcenter_scaled = zcenter * z_factor;
        #[inline(always)]
        fn read_scaled(
            input: &Raster,
            band: isize,
            row: isize,
            col: isize,
            z_factor: f64,
            nodata: f64,
            zcenter_scaled: f64,
        ) -> f64 {
            let v = input.get(band, row, col);
            if v == nodata {
                zcenter_scaled
            } else {
                v * z_factor
            }
        }

        let mut z = [0.0f64; 25];
        z[0] = read_scaled(input, band, row - 2, col - 2, z_factor, nodata, zcenter_scaled);
        z[1] = read_scaled(input, band, row - 2, col - 1, z_factor, nodata, zcenter_scaled);
        z[2] = read_scaled(input, band, row - 2, col, z_factor, nodata, zcenter_scaled);
        z[3] = read_scaled(input, band, row - 2, col + 1, z_factor, nodata, zcenter_scaled);
        z[4] = read_scaled(input, band, row - 2, col + 2, z_factor, nodata, zcenter_scaled);
        z[5] = read_scaled(input, band, row - 1, col - 2, z_factor, nodata, zcenter_scaled);
        z[6] = read_scaled(input, band, row - 1, col - 1, z_factor, nodata, zcenter_scaled);
        z[7] = read_scaled(input, band, row - 1, col, z_factor, nodata, zcenter_scaled);
        z[8] = read_scaled(input, band, row - 1, col + 1, z_factor, nodata, zcenter_scaled);
        z[9] = read_scaled(input, band, row - 1, col + 2, z_factor, nodata, zcenter_scaled);
        z[10] = read_scaled(input, band, row, col - 2, z_factor, nodata, zcenter_scaled);
        z[11] = read_scaled(input, band, row, col - 1, z_factor, nodata, zcenter_scaled);
        z[12] = read_scaled(input, band, row, col, z_factor, nodata, zcenter_scaled);
        z[13] = read_scaled(input, band, row, col + 1, z_factor, nodata, zcenter_scaled);
        z[14] = read_scaled(input, band, row, col + 2, z_factor, nodata, zcenter_scaled);
        z[15] = read_scaled(input, band, row + 1, col - 2, z_factor, nodata, zcenter_scaled);
        z[16] = read_scaled(input, band, row + 1, col - 1, z_factor, nodata, zcenter_scaled);
        z[17] = read_scaled(input, band, row + 1, col, z_factor, nodata, zcenter_scaled);
        z[18] = read_scaled(input, band, row + 1, col + 1, z_factor, nodata, zcenter_scaled);
        z[19] = read_scaled(input, band, row + 1, col + 2, z_factor, nodata, zcenter_scaled);
        z[20] = read_scaled(input, band, row + 2, col - 2, z_factor, nodata, zcenter_scaled);
        z[21] = read_scaled(input, band, row + 2, col - 1, z_factor, nodata, zcenter_scaled);
        z[22] = read_scaled(input, band, row + 2, col, z_factor, nodata, zcenter_scaled);
        z[23] = read_scaled(input, band, row + 2, col + 1, z_factor, nodata, zcenter_scaled);
        z[24] = read_scaled(input, band, row + 2, col + 2, z_factor, nodata, zcenter_scaled);
        Some(z)
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn pq_projected_precomputed_scale(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
        projected_scale: f64,
    ) -> Option<(f64, f64)> {
        let z = Self::neighbourhood_5x5(input, band, row, col, z_factor)?;

        let p = projected_scale
            * (44.0 * (z[3] + z[23] - z[1] - z[21])
                + 31.0
                    * (z[0] + z[20] - z[4] - z[24]
                        + 2.0 * (z[8] + z[18] - z[6] - z[16]))
                + 17.0 * (z[14] - z[10] + 4.0 * (z[13] - z[11]))
                + 5.0 * (z[9] + z[19] - z[5] - z[15]));

        let q = projected_scale
            * (44.0 * (z[5] + z[9] - z[15] - z[19])
                + 31.0
                    * (z[20] + z[24] - z[0] - z[4]
                        + 2.0 * (z[6] + z[8] - z[16] - z[18]))
                + 17.0 * (z[2] - z[22] + 4.0 * (z[7] - z[17]))
                + 5.0 * (z[1] + z[3] - z[21] - z[23]));

        Some((p, q))
    }

    #[inline(always)]
    fn pq_projected(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
        dx: f64,
        dy: f64,
    ) -> Option<(f64, f64)> {
        // Use 5x5 Florinsky method for projected coordinates to match legacy behavior.
        // From Florinsky (2016) Principles and Methods of Digital Terrain Modelling, Chapter 4, pg. 117.
        let z = Self::neighbourhood_5x5(input, band, row, col, z_factor)?;
        let res = (dx + dy) / 2.0;
        
        let p = 1.0 / (420.0 * res)
            * (44.0 * (z[3] + z[23] - z[1] - z[21])
                + 31.0
                    * (z[0] + z[20] - z[4] - z[24]
                        + 2.0 * (z[8] + z[18] - z[6] - z[16]))
                + 17.0 * (z[14] - z[10] + 4.0 * (z[13] - z[11]))
                + 5.0 * (z[9] + z[19] - z[5] - z[15]));

        let q = 1.0 / (420.0 * res)
            * (44.0 * (z[5] + z[9] - z[15] - z[19])
                + 31.0
                    * (z[20] + z[24] - z[0] - z[4]
                        + 2.0 * (z[6] + z[8] - z[16] - z[18]))
                + 17.0 * (z[2] - z[22] + 4.0 * (z[7] - z[17]))
                + 5.0 * (z[1] + z[3] - z[21] - z[23]));

        Some((p, q))
    }

    #[inline(always)]
    fn pq_geographic(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
    ) -> Option<(f64, f64)> {
        let z = Self::neighbourhood(input, band, row, col, z_factor)?;

        let phi1 = input.row_center_y(row);
        let lambda1 = input.col_center_x(col);
        let b = Self::haversine_distance_m(phi1, lambda1, phi1, input.col_center_x(col - 1))
            .max(f64::EPSILON);
        let d = Self::haversine_distance_m(phi1, lambda1, input.row_center_y(row + 1), lambda1)
            .max(f64::EPSILON);
        let e = Self::haversine_distance_m(phi1, lambda1, input.row_center_y(row - 1), lambda1)
            .max(f64::EPSILON);
        let a = Self::haversine_distance_m(
            input.row_center_y(row + 1),
            input.col_center_x(col),
            input.row_center_y(row + 1),
            input.col_center_x(col - 1),
        )
        .max(f64::EPSILON);
        let c = Self::haversine_distance_m(
            input.row_center_y(row - 1),
            input.col_center_x(col),
            input.row_center_y(row - 1),
            input.col_center_x(col - 1),
        )
        .max(f64::EPSILON);

        let a2 = a * a;
        let b2 = b * b;
        let c2 = c * c;
        let d2 = d * d;
        let e2 = e * e;
        let de = d + e;

        let p = (a2 * c * d * de * (z[2] - z[0])
            + b * (a2 * d2 + c2 * e2) * (z[5] - z[3])
            + a * c2 * e * de * (z[8] - z[6]))
            / (2.0 * (a2 * c2 * de * de + b2 * (a2 * d2 + c2 * e2)));

        let q = 1.0 / (3.0 * d * e * de * (a2 * a2 + b2 * b2 + c2 * c2))
            * ((d2 * (a2 * a2 + b2 * b2 + b2 * c2) + c2 * e2 * (a2 - b2)) * (z[0] + z[2])
                - (d2 * (a2 * a2 + c2 * c2 + b2 * c2) - e2 * (a2 * a2 + c2 * c2 + a2 * b2))
                    * (z[3] + z[5])
                - (e2 * (b2 * b2 + c2 * c2 + a2 * b2) - a2 * d2 * (b2 - c2)) * (z[6] + z[8])
                + d2 * (b2 * b2 * (z[1] - 3.0 * z[4])
                    + c2 * c2 * (3.0 * z[1] - z[4])
                    + (a2 * a2 - 2.0 * b2 * c2) * (z[1] - z[4]))
                + e2 * (a2 * a2 * (z[4] - 3.0 * z[7])
                    + b2 * b2 * (3.0 * z[4] - z[7])
                    + (c2 * c2 - 2.0 * a2 * b2) * (z[4] - z[7]))
                - 2.0 * (a2 * d2 * (b2 - c2) * z[7] + c2 * e2 * (a2 - b2) * z[1]));

        Some((p, q))
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
        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator.clone()));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    fn slope_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "slope",
            display_name: "Slope",
            summary: r#"Computes slope gradient from digital elevation model (DEM) using Zevenbergen-Thorne method over 3×3 moving window. Outputs in degrees (0-90), radians (0-π/2), or percent-slope (0-∞). Essential first-step terrain characterization tool used in hydrology, geomorphology, solar radiation modeling, and erosion assessment. Steep slopes (>30°) indicate mountains/ridges; gentle slopes (<5°) indicate plains/valleys.

Slope is the most fundamental geomorphometric parameter, essential for downstream terrain analysis, landslide susceptibility mapping, flow path routing, and aspect-slope correlations. Output directly feeds into curvature calculations, flow direction algorithms, hillshading, and visibility analysis. Z-factor parameter allows compensation for vertical exaggeration or coordinate system differences (e.g., converting projected-meter slope to geographic-degree slope).

Common workflow: (1) Slope magnitude for terrain classification, (2) Combined with aspect for sunlight exposure analysis, (3) Input to curvature for concavity/convexity mapping, (4) Thresholded for steep-slope hazard mapping. Compare to Tangential Curvature (flow-divergence indicator) and Profile Curvature (acceleration/deceleration). Percent-slope output useful for comparison to hydraulic gradient in hydrologic analysis."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "units", description: "Output units: degrees, radians, percent.", required: false },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn slope_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("units".to_string(), json!("degrees"));
        defaults.insert("z_factor".to_string(), json!(1.0));

        ToolManifest {
            id: "slope".to_string(),
            display_name: "Slope".to_string(),
            summary: r#"Zevenbergen-Thorne slope gradient (degrees/radians/percent). Fundamental geomorphometric metric; downstream input to curvature, flow direction, shading, visibility."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "units".to_string(), description: "Output units: degrees, radians, percent.".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Z conversion factor.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_slope".to_string(), description: "Slope in degrees.".to_string(), args: ToolArgs::new() }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "slope".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn run_slope(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let z_factor = Self::parse_z_factor(args);
        let units = args
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("degrees")
            .to_ascii_lowercase();
        if units != "degrees" && units != "radians" && units != "percent" {
            return Err(ToolError::Validation(
                "parameter 'units' must be one of: degrees, radians, percent".to_string(),
            ));
        }

        let input = Self::load_raster(&input_path)?;
        let mut output = Raster::new(RasterConfig {
            cols: input.cols,
            rows: input.rows,
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
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let dx = input.cell_size_x.abs().max(f64::EPSILON);
        let dy = input.cell_size_y.abs().max(f64::EPSILON);
        let is_geographic = Self::raster_is_geographic(&input);

        let band_stride = rows * cols;
        output.data.par_fill_with(|i| {
            let band = (i / band_stride) as isize;
            let rc = i % band_stride;
            let row = (rc / cols) as isize;
            let col = (rc % cols) as isize;
            let Some((p, q)) = (if is_geographic {
                Self::pq_geographic(&input, band, row, col, z_factor)
            } else {
                Self::pq_projected(&input, band, row, col, z_factor, dx, dy)
            }) else {
                return nodata;
            };
            let t = p.mul_add(p, q * q).sqrt();
            match units.as_str() {
                "radians" => t.atan(),
                "percent" => t * 100.0,
                _ => t.atan().to_degrees(),
            }
        });
        ctx.progress.progress(1.0);

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn aspect_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "aspect",
            display_name: "Aspect",
            summary: r#"Calculates slope aspect (direction of maximum slope) in degrees clockwise from north (0-360). Output ranges: 0° (north-facing), 90° (east-facing), 180° (south-facing), 270° (west-facing). Computed as the direction of the downslope gradient vector using Zevenbergen-Thorne method over 3×3 window.

Aspect is critical for solar radiation modeling, predicting vegetation patterns (sunlight exposure), microclimate assessment, snow melt timing, and landslide susceptibility in mountainous terrain. North-facing slopes receive less solar radiation (cooler, wetter); south-facing slopes receive more (hotter, drier). Combined with slope magnitude for comprehensive exposure characterization.

Applications: (1) Solar potential analysis, (2) Ecological habitat classification (aspect-dependent vegetation), (3) Avalanche forecasting (slope + aspect + snow), (4) Vineyard yield prediction (aspect affects ripening), (5) Erosion models (aspect affects water/wind), (6) Combined with slope for terrain classification. Note: Aspect undefined on flat terrain (slope ≈ 0); such cells should be masked before interpretation."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn aspect_manifest() -> ToolManifest {
        ToolManifest {
            id: "aspect".to_string(),
            display_name: "Aspect".to_string(),
            summary: r#"Direction of maximum slope (0°=N, 90°=E, 180°=S, 270°=W). Critical for solar radiation, vegetation patterns, microclimate, and exposure analysis."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "aspect".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn run_aspect(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let z_factor = Self::parse_z_factor(args);

        let input = Self::load_raster(&input_path)?;
        let mut output = Raster::new_like(&input);
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let dx = input.cell_size_x.abs().max(f64::EPSILON);
        let dy = input.cell_size_y.abs().max(f64::EPSILON);
        let is_geographic = Self::raster_is_geographic(&input);

        let band_stride = rows * cols;
        output.data.par_fill_with(|i| {
            let band = (i / band_stride) as isize;
            let rc = i % band_stride;
            let row = (rc / cols) as isize;
            let col = (rc % cols) as isize;
            let Some((p, q)) = (if is_geographic {
                Self::pq_geographic(&input, band, row, col, z_factor)
            } else {
                Self::pq_projected(&input, band, row, col, z_factor, dx, dy)
            }) else {
                return nodata;
            };
            let g = p.mul_add(p, q * q).sqrt();
            if g <= 0.0 {
                -1.0
            } else {
                let mut aspect = 180.0 - (q / p).atan().to_degrees() + 90.0 * p.signum();
                if aspect >= 360.0 {
                    aspect -= 360.0;
                }
                aspect
            }
        });
        ctx.progress.progress(1.0);

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn convergence_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "convergence_index",
            display_name: "Convergence Index",
            summary: r#"Calculates convergence (flow concentration) / divergence (flow dispersal) index from local neighbor aspect alignment. Positive values (convergent) indicate flow concentration toward valley floor; negative values (divergent) indicate flow dispersal on ridges. Ranges typically -1 (highly divergent) to +1 (highly convergent). Identifies concentrated vs. dispersed flow zones.

Convergence Index measures how aligned neighboring cell aspects are—high alignment indicates concave (convergent) terrain; low/misaligned indicates convex (divergent) terrain. Particularly useful for identifying drainage lines, ridge crests, and transition zones without computing full flow routing. Computationally efficient alternative to complex flow algorithms.

Applications: (1) Valley/ridge identification without flow direction computation, (2) Landslide susceptibility (convergent zones are wet, unstable), (3) Vegetation pattern mapping (convergent zones wetter), (4) Hydrogeomorphic unit mapping (convergent = valleys; divergent = ridges), (5) Erosion modeling (concentrated flow indicates higher erosion). Often combined with slope magnitude: steep + convergent = gully zones; gentle + divergent = knoll zones."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn convergence_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("z_factor".to_string(), json!(1.0));

        ToolManifest {
            id: "convergence_index".to_string(),
            display_name: "Convergence Index".to_string(),
            summary: r#"Flow convergence/divergence from local aspect alignment. Identifies valleys (convergent) and ridges (divergent) without full flow routing. Efficient alternative to flow direction algorithms."#
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "convergence".to_string(),
            ],
            stability: ToolStability::Stable,
        }
    }

    fn run_convergence_index(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let z_factor = Self::parse_z_factor(args);

        let input = Self::load_raster(&input_path)?;
        let mut output = Raster::new_like(&input);
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let coalescer = PercentCoalescer::new(1, 99);
        let nodata = input.nodata;
        let dx = input.cell_size_x.abs().max(f64::EPSILON);
        let dy = input.cell_size_y.abs().max(f64::EPSILON);
        let is_geographic = Self::raster_is_geographic(&input);

        let offsets = [
            (-1isize, -1isize),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        let azimuth = [135.0f64, 180.0, 225.0, 90.0, 270.0, 45.0, 0.0, 315.0];

        for band_idx in 0..bands {
            let band = band_idx as isize;

            // Stage 1: aspect raster in degrees clockwise from north.
            // Use f32 backing to reduce peak memory while preserving precision for angle comparisons.
            let mut aspect = vec![nodata as f32; rows * cols];
            aspect
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    for (c, cell) in row_out.iter_mut().enumerate() {
                        let row = r as isize;
                        let col = c as isize;
                        let Some((p, q)) = (if is_geographic {
                            Self::pq_geographic(&input, band, row, col, z_factor)
                        } else {
                            Self::pq_projected(&input, band, row, col, z_factor, dx, dy)
                        }) else {
                            continue;
                        };

                        let g = p.mul_add(p, q * q).sqrt();
                        let aspect_deg = if g <= 0.0 {
                            -1.0
                        } else {
                            let mut aspect = 180.0 - (q / p).atan().to_degrees() + 90.0 * p.signum();
                            if aspect >= 360.0 {
                                aspect -= 360.0;
                            }
                            aspect
                        };
                        *cell = aspect_deg as f32;
                    }
                });
            let aspect = Arc::new(aspect);

            // Stage 2: convergence index from neighbour aspect alignment.
            let mut conv = vec![nodata; rows * cols];
            conv.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                for (c, cell) in row_out.iter_mut().enumerate() {
                    let i = r * cols + c;
                    if aspect[i] as f64 == nodata {
                        continue;
                    }
                    let mut sum = 0.0;
                    let mut n = 0.0;
                    for k in 0..8 {
                        let rr = r as isize + offsets[k].1;
                        let cc = c as isize + offsets[k].0;
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let a = aspect[rr as usize * cols + cc as usize] as f64;
                        if a == nodata {
                            continue;
                        }
                        let mut rel = (a - azimuth[k]).abs();
                        if rel > 180.0 {
                            rel = 360.0 - rel;
                        }
                        sum += rel;
                        n += 1.0;
                    }
                    if n > 0.0 {
                        *cell = sum / n - 90.0;
                    }
                }
            });

            for r in 0..rows {
                let start = r * cols;
                let end = start + cols;
                output
                    .set_row_slice(band, r as isize, &conv[start..end])
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }

            coalescer.emit_unit_fraction(ctx.progress, (band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn hillshade_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "hillshade",
            display_name: "Hillshade",
            summary: r#"Produces shaded relief (hillshade) visualization from DEM using directional illumination model. Illumination from specified azimuth (0-360°) and altitude (0-90°) angles creates 3D appearance showing terrain relief. Output is grayscale 0-255 image where slopes facing light appear bright, slopes facing away appear dark.

Hillshade is fundamental terrain visualization technique for qualitative relief display and analysis. Single light source variant (this tool) is fast, simple, and suitable for quick DEM inspection, map production, and terrain characterization. Azimuth parameter controls light direction (0°=North, 90°=East, 180°=South, 270°=West); altitude controls light angle above horizon (90°=overhead, 45°=typical, 0°=horizon grazing). Z-factor exaggerates relief for better visualization on gentle terrain.

Applications: (1) DEM quality assessment (visual inspection for artifacts), (2) Terrain visualization for reports/publications, (3) Overlay on satellite/map data for 3D context, (4) Manual landform interpretation, (5) Rapid terrain characterization. For enhanced feature visibility with reduced shadow artifacts, use Multidirectional Hillshade (combines multiple light sources)."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
        }
    }

    fn hillshade_manifest() -> ToolManifest {
        ToolManifest {
            id: "hillshade".to_string(),
            display_name: "Hillshade".to_string(),
            summary: r#"Single-source directional hillshade visualization (grayscale 0-255). Azimuth & altitude parameters control light direction. Fast terrain visualization for DEM inspection and map display."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hillshade".to_string(), "render_hint:raster=grayscale_brightness".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn run_hillshade(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Self::run_shade_core(args, ctx, false)
    }

    fn multidirectional_hillshade_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multidirectional_hillshade",
            display_name: "Multidirectional Hillshade",
            summary: r#"Produces weighted multi-directional shaded relief combining illumination from multiple azimuth angles (typically 8 directions) to enhance feature visibility. Reduces shadow artifacts present in single-light hillshade by averaging contributions from multiple light sources. Output is grayscale 0-255 image where all terrain features are similarly enhanced regardless of light direction.

Multidirectional hillshade overcomes single-light shadowing limitations: features perpendicular to single light source direction become nearly invisible. By combining multiple light sources, all features become visible, reducing perceptual bias toward specific terrain aspects. Particularly effective for complex terrain with ridges and valleys in varied directions. Slightly slower than single-light hillshade but produces more balanced relief visualization.

Applications: (1) Enhanced DEM visualization (better than single-light for complex terrain), (2) Terrain visualization for publications/presentations (more professional appearance), (3) Improved manual landform interpretation (no directional shadowing bias), (4) Combined with satellite/color data (produces superior multispectral overlays), (5) Quality assessment of DEMs (artifacts more visible under multi-directional lighting). Preferred over single Hillshade for final visualization products and detailed terrain analysis."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
        }
    }

    fn multidirectional_hillshade_manifest() -> ToolManifest {
        ToolManifest {
            id: "multidirectional_hillshade".to_string(),
            display_name: "Multidirectional Hillshade".to_string(),
            summary: r#"Multi-directional hillshade (8+ light sources) eliminating single-light shadowing artifacts. Balanced feature visibility for complex terrain; preferred for publications and detailed analysis."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hillshade".to_string(), "render_hint:raster=grayscale_brightness".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn run_multidirectional_hillshade(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        Self::run_shade_core(args, ctx, true)
    }

    fn run_shade_core(
        args: &ToolArgs,
        ctx: &ToolContext,
        multi: bool,
    ) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let z_factor = Self::parse_z_factor(args);
        let altitude_deg = args.get("altitude").and_then(|v| v.as_f64()).unwrap_or(30.0);
        let full_360_mode = if multi {
            args.get("full_360_mode")
                .or_else(|| args.get("full_360"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        } else {
            false
        };

        let azimuth_single = args.get("azimuth").and_then(|v| v.as_f64()).unwrap_or(315.0);
        let altitude_rad = altitude_deg.to_radians();
        let sin_alt = altitude_rad.sin();
        let cos_alt = altitude_rad.cos();

        let azimuths_4 = [
            (225.0_f64 - 90.0).to_radians(),
            (270.0_f64 - 90.0).to_radians(),
            (315.0_f64 - 90.0).to_radians(),
            (360.0_f64 - 90.0).to_radians(),
        ];
        let weights_4 = [0.1, 0.4, 0.4, 0.1];
        let azimuths_8 = [
            (0.0_f64 - 90.0).to_radians(),
            (45.0_f64 - 90.0).to_radians(),
            (90.0_f64 - 90.0).to_radians(),
            (135.0_f64 - 90.0).to_radians(),
            (180.0_f64 - 90.0).to_radians(),
            (225.0_f64 - 90.0).to_radians(),
            (270.0_f64 - 90.0).to_radians(),
            (315.0_f64 - 90.0).to_radians(),
        ];
        let weights_8 = [0.15, 0.125, 0.1, 0.05, 0.1, 0.125, 0.15, 0.2];

        let single_az = [(azimuth_single - 90.0).to_radians()];
        let single_w = [1.0_f64];

        let (azimuths, weights): (&[f64], &[f64]) = if !multi {
            (&single_az, &single_w)
        } else if full_360_mode {
            (&azimuths_8, &weights_8)
        } else {
            (&azimuths_4, &weights_4)
        };

        let input = Self::load_raster(&input_path)?;
        let mut output = Raster::new_like(&input);
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let coalescer = PercentCoalescer::new(1, 99);
        let nodata = input.nodata;
        let dx = input.cell_size_x.abs().max(f64::EPSILON);
        let dy = input.cell_size_y.abs().max(f64::EPSILON);
        let projected_scale = 1.0 / (420.0 * ((dx + dy) / 2.0));
        let is_geographic = Self::raster_is_geographic(&input);

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut band_out = vec![nodata; rows * cols];

            if is_geographic {
                band_out
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(r, row_out)| {
                        for (c, cell) in row_out.iter_mut().enumerate() {
                            let row = r as isize;
                            let col = c as isize;
                            let Some((p, q)) = Self::pq_geographic(&input, band, row, col, z_factor) else {
                                continue;
                            };
                            let tan_slope = p.mul_add(p, q * q).sqrt().max(0.00017);
                            let aspect = if p != 0.0 {
                                std::f64::consts::PI
                                    - (q / p).atan()
                                    + std::f64::consts::FRAC_PI_2 * p.signum()
                            } else {
                                std::f64::consts::PI
                            };
                            let term1 = tan_slope / (1.0 + tan_slope * tan_slope).sqrt();
                            let term2 = sin_alt / tan_slope;
                            let mut val = 0.0;
                            for i in 0..azimuths.len() {
                                let term3 = cos_alt * (azimuths[i] - aspect).sin();
                                val += term1 * (term2 - term3) * weights[i];
                            }
                            *cell = (val * 32767.0).max(0.0).round();
                        }
                    });
            } else {
                let mut band_buf = vec![nodata; rows * cols];
                band_buf
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(r, row_buf)| {
                        for (c, cell) in row_buf.iter_mut().enumerate() {
                            *cell = input.get(band, r as isize, c as isize);
                        }
                    });
                let band_buf = Arc::new(band_buf);

                band_out
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(r, row_out)| {
                        let row = r as isize;
                        for (c, cell) in row_out.iter_mut().enumerate() {
                            let idx = r * cols + c;
                            let z_center = band_buf[idx];
                            if z_center == nodata {
                                continue;
                            }
                            let z_center_scaled = z_center * z_factor;
                            let read_scaled = |rr: isize, cc: isize| -> f64 {
                                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                    return z_center_scaled;
                                }
                                let v = band_buf[rr as usize * cols + cc as usize];
                                if v == nodata {
                                    z_center_scaled
                                } else {
                                    v * z_factor
                                }
                            };

                            let col = c as isize;
                            let z0 = read_scaled(row - 2, col - 2);
                            let z1 = read_scaled(row - 2, col - 1);
                            let z2 = read_scaled(row - 2, col);
                            let z3 = read_scaled(row - 2, col + 1);
                            let z4 = read_scaled(row - 2, col + 2);
                            let z5 = read_scaled(row - 1, col - 2);
                            let z6 = read_scaled(row - 1, col - 1);
                            let z7 = read_scaled(row - 1, col);
                            let z8 = read_scaled(row - 1, col + 1);
                            let z9 = read_scaled(row - 1, col + 2);
                            let z10 = read_scaled(row, col - 2);
                            let z11 = read_scaled(row, col - 1);
                            let z13 = read_scaled(row, col + 1);
                            let z14 = read_scaled(row, col + 2);
                            let z15 = read_scaled(row + 1, col - 2);
                            let z16 = read_scaled(row + 1, col - 1);
                            let z17 = read_scaled(row + 1, col);
                            let z18 = read_scaled(row + 1, col + 1);
                            let z19 = read_scaled(row + 1, col + 2);
                            let z20 = read_scaled(row + 2, col - 2);
                            let z21 = read_scaled(row + 2, col - 1);
                            let z22 = read_scaled(row + 2, col);
                            let z23 = read_scaled(row + 2, col + 1);
                            let z24 = read_scaled(row + 2, col + 2);

                            let p = projected_scale
                                * (44.0 * (z3 + z23 - z1 - z21)
                                    + 31.0
                                        * (z0 + z20 - z4 - z24
                                            + 2.0 * (z8 + z18 - z6 - z16))
                                    + 17.0 * (z14 - z10 + 4.0 * (z13 - z11))
                                    + 5.0 * (z9 + z19 - z5 - z15));

                            let q = projected_scale
                                * (44.0 * (z5 + z9 - z15 - z19)
                                    + 31.0
                                        * (z20 + z24 - z0 - z4
                                            + 2.0 * (z6 + z8 - z16 - z18))
                                    + 17.0 * (z2 - z22 + 4.0 * (z7 - z17))
                                    + 5.0 * (z1 + z3 - z21 - z23));

                            let tan_slope = p.mul_add(p, q * q).sqrt().max(0.00017);
                            let aspect = if p != 0.0 {
                                std::f64::consts::PI
                                    - (q / p).atan()
                                    + std::f64::consts::FRAC_PI_2 * p.signum()
                            } else {
                                std::f64::consts::PI
                            };
                            let term1 = tan_slope / (1.0 + tan_slope * tan_slope).sqrt();
                            let term2 = sin_alt / tan_slope;
                            let mut val = 0.0;
                            for i in 0..azimuths.len() {
                                let term3 = cos_alt * (azimuths[i] - aspect).sin();
                                val += term1 * (term2 - term3) * weights[i];
                            }
                            *cell = (val * 32767.0).max(0.0).round();
                        }
                    });
            }

            for r in 0..rows {
                let start = r * cols;
                let end = start + cols;
                output
                    .set_row_slice(band, r as isize, &band_out[start..end])
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            coalescer.emit_unit_fraction(ctx.progress, (band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }
}

impl Tool for SlopeTool {
    fn metadata(&self) -> ToolMetadata { TerrainCore::slope_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainCore::slope_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainCore::run_slope(args, ctx)
    }
}

impl Tool for AspectTool {
    fn metadata(&self) -> ToolMetadata { TerrainCore::aspect_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainCore::aspect_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainCore::run_aspect(args, ctx)
    }
}

impl Tool for ConvergenceIndexTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainCore::convergence_index_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainCore::convergence_index_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainCore::run_convergence_index(args, ctx)
    }
}

impl Tool for HillshadeTool {
    fn metadata(&self) -> ToolMetadata { TerrainCore::hillshade_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainCore::hillshade_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainCore::run_hillshade(args, ctx)
    }
}

impl Tool for MultidirectionalHillshadeTool {
    fn metadata(&self) -> ToolMetadata { TerrainCore::multidirectional_hillshade_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainCore::multidirectional_hillshade_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainCore::run_multidirectional_hillshade(args, ctx)
    }
}
