use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::json;
use wbprojection::{identify_epsg_from_wkt_with_policy, Crs, EpsgIdentifyPolicy};
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};

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

    fn pq_projected(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
        dx: f64,
        dy: f64,
    ) -> Option<(f64, f64)> {
        let z = Self::neighbourhood(input, band, row, col, z_factor)?;
        let p = (z[5] - z[3]) / (2.0 * dx);
        let q = (z[1] - z[7]) / (2.0 * dy);
        Some((p, q))
    }

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
            summary: "Calculates slope gradient from a DEM.",
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
            summary: "Calculates slope gradient from a DEM.".to_string(),
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
            stability: ToolStability::Experimental,
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
            summary: "Calculates slope aspect in degrees clockwise from north.",
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
            summary: "Calculates slope aspect in degrees clockwise from north.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "aspect".to_string()],
            stability: ToolStability::Experimental,
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
            summary: "Calculates the convergence/divergence index from local neighbour aspect alignment.",
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
            summary: "Calculates the convergence/divergence index from local neighbour aspect alignment."
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
            stability: ToolStability::Experimental,
        }
    }

    fn run_convergence_index(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let z_factor = Self::parse_z_factor(args);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
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
            let aspect_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
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
                        row_out[c] = if g <= 0.0 {
                            -1.0
                        } else {
                            let mut aspect = 180.0 - (q / p).atan().to_degrees() + 90.0 * p.signum();
                            if aspect >= 360.0 {
                                aspect -= 360.0;
                            }
                            aspect
                        };
                    }
                    row_out
                })
                .collect();

            let mut aspect = vec![nodata; rows * cols];
            for (r, row_vals) in aspect_rows.iter().enumerate() {
                for c in 0..cols {
                    aspect[r * cols + c] = row_vals[c];
                }
            }

            // Stage 2: convergence index from neighbour aspect alignment.
            let conv_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let i = r * cols + c;
                        if aspect[i] == nodata {
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
                            let a = aspect[rr as usize * cols + cc as usize];
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
                            row_out[c] = sum / n - 90.0;
                        }
                    }
                    row_out
                })
                .collect();

            for (r, row_vals) in conv_rows.iter().enumerate() {
                output
                    .set_row_slice(band, r as isize, row_vals)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }

            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn hillshade_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "hillshade",
            display_name: "Hillshade",
            summary: "Produces shaded-relief from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
        }
    }

    fn hillshade_manifest() -> ToolManifest {
        ToolManifest {
            id: "hillshade".to_string(),
            display_name: "Hillshade".to_string(),
            summary: "Produces shaded-relief from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hillshade".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_hillshade(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Self::run_shade_core(args, ctx, false)
    }

    fn multidirectional_hillshade_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multidirectional_hillshade",
            display_name: "Multidirectional Hillshade",
            summary: "Produces weighted multi-azimuth shaded-relief.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
        }
    }

    fn multidirectional_hillshade_manifest() -> ToolManifest {
        ToolManifest {
            id: "multidirectional_hillshade".to_string(),
            display_name: "Multidirectional Hillshade".to_string(),
            summary: "Produces weighted multi-azimuth shaded-relief.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hillshade".to_string()],
            stability: ToolStability::Experimental,
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
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let dx = input.cell_size_x.abs().max(f64::EPSILON);
        let dy = input.cell_size_y.abs().max(f64::EPSILON);
        let is_geographic = Self::raster_is_geographic(&input);

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let row = r as isize;
                        let col = c as isize;
                        let Some((p, q)) = (if is_geographic {
                            Self::pq_geographic(&input, band, row, col, z_factor)
                        } else {
                            Self::pq_projected(&input, band, row, col, z_factor, dx, dy)
                        }) else {
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
                        row_out[c] = (val * 32767.0).max(0.0).round();
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output
                    .set_row_slice(band, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
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
