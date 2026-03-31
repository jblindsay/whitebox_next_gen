use std::collections::BTreeMap;
use std::f64::consts::PI;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::color_math::{hsi2value, value2hsi, value2i};
use wbraster::{Raster, RasterFormat};

use super::color_support;
use crate::memory_store;

pub struct AnisotropicDiffusionFilterTool;
pub struct GammaCorrectionTool;
pub struct GuidedFilterTool;
pub struct WienerFilterTool;
pub struct NonLocalMeansFilterTool;
pub struct KuwaharaFilterTool;
pub struct FrostFilterTool;
pub struct GammaMapFilterTool;
pub struct KuanFilterTool;
pub struct GaborFilterBankTool;
pub struct FrangiFilterTool;
pub struct SavitzkyGolay2dFilterTool;

#[derive(Clone, Copy)]
enum AdvancedOp {
    AnisotropicDiffusion,
    GammaCorrection,
    Guided,
    Wiener,
    NonLocalMeans,
    Kuwahara,
    Frost,
    GammaMap,
    Kuan,
    Gabor,
    Frangi,
    SavitzkyGolay2d,
}

impl AdvancedOp {
    fn id(self) -> &'static str {
        match self {
            Self::AnisotropicDiffusion => "anisotropic_diffusion_filter",
            Self::GammaCorrection => "gamma_correction",
            Self::Guided => "guided_filter",
            Self::Wiener => "wiener_filter",
            Self::NonLocalMeans => "non_local_means_filter",
            Self::Kuwahara => "kuwahara_filter",
            Self::Frost => "frost_filter",
            Self::GammaMap => "gamma_map_filter",
            Self::Kuan => "kuan_filter",
            Self::Gabor => "gabor_filter_bank",
            Self::Frangi => "frangi_filter",
            Self::SavitzkyGolay2d => "savitzky_golay_2d_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::AnisotropicDiffusion => "Anisotropic Diffusion Filter",
            Self::GammaCorrection => "Gamma Correction",
            Self::Guided => "Guided Filter",
            Self::Wiener => "Wiener Filter",
            Self::NonLocalMeans => "Non-Local Means Filter",
            Self::Kuwahara => "Kuwahara Filter",
            Self::Frost => "Frost Filter",
            Self::GammaMap => "Gamma-MAP Filter",
            Self::Kuan => "Kuan Filter",
            Self::Gabor => "Gabor Filter Bank",
            Self::Frangi => "Frangi Filter",
            Self::SavitzkyGolay2d => "Savitzky-Golay 2D Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::AnisotropicDiffusion => {
                "Performs Perona-Malik edge-preserving anisotropic diffusion smoothing."
            }
            Self::GammaCorrection => "Applies gamma intensity correction to grayscale or RGB imagery.",
            Self::Guided => "Performs edge-preserving guided filtering using local linear models.",
            Self::Wiener => "Performs adaptive Wiener denoising using local mean and variance.",
            Self::NonLocalMeans => "Performs non-local means denoising using patch similarity weighting.",
            Self::Kuwahara => "Performs edge-preserving Kuwahara filtering using minimum-variance subwindows.",
            Self::Frost => "Performs adaptive Frost speckle filtering for radar imagery.",
            Self::GammaMap => "Performs Gamma-MAP speckle filtering for radar imagery.",
            Self::Kuan => "Performs Kuan speckle filtering for radar imagery.",
            Self::Gabor => "Performs multi-orientation Gabor response filtering.",
            Self::Frangi => "Performs multiscale Frangi vesselness enhancement.",
            Self::SavitzkyGolay2d => "Performs 2D Savitzky-Golay smoothing.",
        }
    }
}

impl AdvancedFilters {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}': store entry is missing",
                    id
                ))
            });
        }

        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
    }

    fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
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

    fn metadata_for(op: AdvancedOp) -> ToolMetadata {
        let mut params = vec![ToolParamSpec {
            name: "input",
            description: "Input raster path or typed raster object.",
            required: true,
        }];

        match op {
            AdvancedOp::AnisotropicDiffusion => {
                params.push(ToolParamSpec {
                    name: "iterations",
                    description: "Number of diffusion iterations (default 10).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "kappa",
                    description: "Edge sensitivity parameter (default 20.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "lambda",
                    description: "Time-step in (0, 0.25], default 0.2.",
                    required: false,
                });
            }
            AdvancedOp::GammaCorrection => {
                params.push(ToolParamSpec {
                    name: "gamma",
                    description: "Gamma exponent in [0, 4], default 0.5.",
                    required: false,
                });
            }
            AdvancedOp::Guided => {
                params.push(ToolParamSpec {
                    name: "radius",
                    description: "Guided filter window radius in pixels (default 4).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "epsilon",
                    description: "Regularization parameter for local variance (default 0.01).",
                    required: false,
                });
            }
            AdvancedOp::Wiener => {
                params.push(ToolParamSpec {
                    name: "radius",
                    description: "Wiener local window radius in pixels (default 2).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "noise_variance",
                    description: "Optional additive noise variance. If omitted, estimated from local variance map.",
                    required: false,
                });
            }
            AdvancedOp::NonLocalMeans => {
                params.push(ToolParamSpec {
                    name: "search_radius",
                    description: "Search window radius in pixels (default 5).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "patch_radius",
                    description: "Patch radius in pixels (default 1).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "h",
                    description: "Filtering strength parameter (default 10.0).",
                    required: false,
                });
            }
            AdvancedOp::Kuwahara => {
                params.push(ToolParamSpec {
                    name: "radius",
                    description: "Kuwahara quadrant radius in pixels (default 2).",
                    required: false,
                });
            }
            AdvancedOp::Frost => {
                params.push(ToolParamSpec {
                    name: "radius",
                    description: "Local window radius in pixels (default 2).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "damping_factor",
                    description: "Frost damping factor controlling exponential decay (default 2.0).",
                    required: false,
                });
            }
            AdvancedOp::GammaMap => {
                params.push(ToolParamSpec {
                    name: "radius",
                    description: "Local window radius in pixels (default 2).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "enl",
                    description: "Equivalent number of looks (default 1.0).",
                    required: false,
                });
            }
            AdvancedOp::Kuan => {
                params.push(ToolParamSpec {
                    name: "radius",
                    description: "Local window radius in pixels (default 2).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "enl",
                    description: "Equivalent number of looks (default 1.0).",
                    required: false,
                });
            }
            AdvancedOp::Gabor => {
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Gaussian envelope sigma in pixels (default 2.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "frequency",
                    description: "Sinusoid spatial frequency in cycles/pixel (default 0.2).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "orientations",
                    description: "Number of orientations in the filter bank (default 6).",
                    required: false,
                });
            }
            AdvancedOp::Frangi => {
                params.push(ToolParamSpec {
                    name: "scales",
                    description: "List of Gaussian-like scales in pixels (default [1.0, 2.0, 3.0]).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "beta",
                    description: "Frangi beta parameter for blob suppression (default 0.5).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "c",
                    description: "Frangi c parameter for structure sensitivity (default 15.0).",
                    required: false,
                });
            }
            AdvancedOp::SavitzkyGolay2d => {
                params.push(ToolParamSpec {
                    name: "window_size",
                    description: "Odd window size (default 5). Currently supports 5 for polynomial order 2.",
                    required: false,
                });
            }
        }

        params.push(ToolParamSpec {
            name: "output",
            description: "Optional output path. If omitted, output remains in memory.",
            required: false,
        });

        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
        }
    }

    fn manifest_for(op: AdvancedOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        match op {
            AdvancedOp::AnisotropicDiffusion => {
                defaults.insert("iterations".to_string(), json!(10));
                defaults.insert("kappa".to_string(), json!(20.0));
                defaults.insert("lambda".to_string(), json!(0.2));
            }
            AdvancedOp::GammaCorrection => {
                defaults.insert("gamma".to_string(), json!(0.5));
            }
            AdvancedOp::Guided => {
                defaults.insert("radius".to_string(), json!(4));
                defaults.insert("epsilon".to_string(), json!(0.01));
            }
            AdvancedOp::Wiener => {
                defaults.insert("radius".to_string(), json!(2));
            }
            AdvancedOp::NonLocalMeans => {
                defaults.insert("search_radius".to_string(), json!(5));
                defaults.insert("patch_radius".to_string(), json!(1));
                defaults.insert("h".to_string(), json!(10.0));
            }
            AdvancedOp::Kuwahara => {
                defaults.insert("radius".to_string(), json!(2));
            }
            AdvancedOp::Frost => {
                defaults.insert("radius".to_string(), json!(2));
                defaults.insert("damping_factor".to_string(), json!(2.0));
            }
            AdvancedOp::GammaMap => {
                defaults.insert("radius".to_string(), json!(2));
                defaults.insert("enl".to_string(), json!(1.0));
            }
            AdvancedOp::Kuan => {
                defaults.insert("radius".to_string(), json!(2));
                defaults.insert("enl".to_string(), json!(1.0));
            }
            AdvancedOp::Gabor => {
                defaults.insert("sigma".to_string(), json!(2.0));
                defaults.insert("frequency".to_string(), json!(0.2));
                defaults.insert("orientations".to_string(), json!(6));
            }
            AdvancedOp::Frangi => {
                defaults.insert("scales".to_string(), json!([1.0, 2.0, 3.0]));
                defaults.insert("beta".to_string(), json!(0.5));
                defaults.insert("c".to_string(), json!(15.0));
            }
            AdvancedOp::SavitzkyGolay2d => {
                defaults.insert("window_size".to_string(), json!(5));
            }
        }

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));

        let params = Self::metadata_for(op)
            .params
            .into_iter()
            .map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            })
            .collect();

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
            defaults,
            examples: vec![ToolExample {
                name: format!("basic_{}", op.id()),
                description: format!("Applies {} to an input raster.", op.id()),
                args: example_args,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "filter".to_string(),
                op.id().to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn set_band_from_values(
        input: &Raster,
        output: &mut Raster,
        band_idx: usize,
        vals: &[f64],
        packed_rgb: bool,
    ) -> Result<(), ToolError> {
        let band = band_idx as isize;
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let mut rows_buf = vec![vec![nodata; cols]; rows];

        rows_buf.par_iter_mut().enumerate().for_each(|(r, out_row)| {
            for c in 0..cols {
                let idx = r * cols + c;
                let v = vals[idx];
                if v == nodata {
                    continue;
                }
                if packed_rgb {
                    let z0 = input.get(band, r as isize, c as isize);
                    let (h, s, _) = value2hsi(z0);
                    out_row[c] = hsi2value(h, s, v);
                } else {
                    out_row[c] = v;
                }
            }
        });

        for (r, row) in rows_buf.iter().enumerate() {
            output
                .set_row_slice(band, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
        }

        Ok(())
    }

    fn run_gamma(input: &Raster, packed_rgb: bool, gamma: f64) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let gamma = gamma.clamp(0.0, 4.0);
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            vals.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if input.is_nodata(z_raw) {
                        continue;
                    }
                    let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                    row_out[c] = z.powf(gamma);
                }
            });
            Self::set_band_from_values(input, &mut out, band_idx, &vals, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_anisotropic(
        input: &Raster,
        packed_rgb: bool,
        iterations: usize,
        kappa: f64,
        lambda: f64,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let lambda = lambda.clamp(1e-6, 0.25);
        let kappa = kappa.max(1e-6);

        let mut out = input.clone();
        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut current = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        current[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            for _ in 0..iterations {
                let mut next = current.clone();
                next.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let z = current[idx];
                        if z == nodata {
                            continue;
                        }

                        let north = if r > 0 { current[(r - 1) * cols + c] } else { nodata };
                        let south = if r + 1 < rows { current[(r + 1) * cols + c] } else { nodata };
                        let west = if c > 0 { current[r * cols + (c - 1)] } else { nodata };
                        let east = if c + 1 < cols { current[r * cols + (c + 1)] } else { nodata };

                        let mut acc = 0.0;
                        if north != nodata {
                            let d = north - z;
                            let c_n = (-(d / kappa).powi(2)).exp();
                            acc += c_n * d;
                        }
                        if south != nodata {
                            let d = south - z;
                            let c_s = (-(d / kappa).powi(2)).exp();
                            acc += c_s * d;
                        }
                        if west != nodata {
                            let d = west - z;
                            let c_w = (-(d / kappa).powi(2)).exp();
                            acc += c_w * d;
                        }
                        if east != nodata {
                            let d = east - z;
                            let c_e = (-(d / kappa).powi(2)).exp();
                            acc += c_e * d;
                        }

                        out_row[c] = z + lambda * acc;
                    }
                });
                current = next;
            }

            Self::set_band_from_values(input, &mut out, band_idx, &current, packed_rgb)?;
        }

        Ok(out)
    }

    fn box_mean_from_integral(data: &[f64], rows: usize, cols: usize, radius: usize, nodata: f64) -> Vec<f64> {
        let stride = cols + 1;
        let mut integral_sum = vec![0.0f64; (rows + 1) * (cols + 1)];
        let mut integral_count = vec![0u32; (rows + 1) * (cols + 1)];

        for r in 0..rows {
            let mut row_sum = 0.0;
            let mut row_count = 0u32;
            let ir = (r + 1) * stride;
            let ir_prev = r * stride;
            for c in 0..cols {
                let z = data[r * cols + c];
                if z != nodata {
                    row_sum += z;
                    row_count += 1;
                }
                let idx = ir + (c + 1);
                integral_sum[idx] = integral_sum[ir_prev + (c + 1)] + row_sum;
                integral_count[idx] = integral_count[ir_prev + (c + 1)] + row_count;
            }
        }

        let mut out = vec![nodata; rows * cols];
        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
            for c in 0..cols {
                let y1 = r.saturating_sub(radius);
                let y2 = (r + radius).min(rows - 1);
                let x1 = c.saturating_sub(radius);
                let x2 = (c + radius).min(cols - 1);

                let a = y1 * stride + x1;
                let b = y1 * stride + (x2 + 1);
                let cc = (y2 + 1) * stride + x1;
                let d = (y2 + 1) * stride + (x2 + 1);
                let n = (integral_count[d] + integral_count[a] - integral_count[b] - integral_count[cc]) as f64;
                if n > 0.0 {
                    let sum = integral_sum[d] + integral_sum[a] - integral_sum[b] - integral_sum[cc];
                    out_row[c] = sum / n;
                }
            }
        });
        out
    }

    fn run_guided(input: &Raster, packed_rgb: bool, radius: usize, epsilon: f64) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let radius = radius.max(1);
        let eps = epsilon.max(1e-12);
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut i_vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        i_vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let mean_i = Self::box_mean_from_integral(&i_vals, rows, cols, radius, nodata);
            let i_sq: Vec<f64> = i_vals
                .iter()
                .map(|&z| if z == nodata { nodata } else { z * z })
                .collect();
            let mean_i_sq = Self::box_mean_from_integral(&i_sq, rows, cols, radius, nodata);

            let mut a = vec![nodata; rows * cols];
            let mut b = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let idx = r * cols + c;
                    let m = mean_i[idx];
                    let m2 = mean_i_sq[idx];
                    if m == nodata || m2 == nodata {
                        continue;
                    }
                    let var = m2 - m * m;
                    let aval = var / (var + eps);
                    a[idx] = aval;
                    b[idx] = m - aval * m;
                }
            }

            let mean_a = Self::box_mean_from_integral(&a, rows, cols, radius, nodata);
            let mean_b = Self::box_mean_from_integral(&b, rows, cols, radius, nodata);

            let mut q = vec![nodata; rows * cols];
            q.par_chunks_mut(cols).enumerate().for_each(|(r, row_q)| {
                for c in 0..cols {
                    let idx = r * cols + c;
                    let z = i_vals[idx];
                    if z == nodata || mean_a[idx] == nodata || mean_b[idx] == nodata {
                        continue;
                    }
                    row_q[c] = mean_a[idx] * z + mean_b[idx];
                }
            });

            Self::set_band_from_values(input, &mut out, band_idx, &q, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_wiener(
        input: &Raster,
        packed_rgb: bool,
        radius: usize,
        noise_variance: Option<f64>,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let radius = radius.max(1);
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let mean = Self::box_mean_from_integral(&vals, rows, cols, radius, nodata);
            let sq: Vec<f64> = vals
                .iter()
                .map(|&z| if z == nodata { nodata } else { z * z })
                .collect();
            let mean_sq = Self::box_mean_from_integral(&sq, rows, cols, radius, nodata);

            let mut local_var = vec![nodata; rows * cols];
            let mut var_sum = 0.0;
            let mut var_n = 0usize;
            for i in 0..local_var.len() {
                if mean[i] == nodata || mean_sq[i] == nodata {
                    continue;
                }
                let v = (mean_sq[i] - mean[i] * mean[i]).max(0.0);
                local_var[i] = v;
                var_sum += v;
                var_n += 1;
            }
            let est_noise = noise_variance.unwrap_or_else(|| {
                if var_n > 0 {
                    var_sum / var_n as f64
                } else {
                    0.0
                }
            });

            let mut filtered = vec![nodata; rows * cols];
            filtered
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let z = vals[idx];
                        if z == nodata || mean[idx] == nodata || local_var[idx] == nodata {
                            continue;
                        }
                        let lv = local_var[idx];
                        let gain = if lv > 0.0 {
                            (lv - est_noise).max(0.0) / lv
                        } else {
                            0.0
                        };
                        row_out[c] = mean[idx] + gain * (z - mean[idx]);
                    }
                });

            Self::set_band_from_values(input, &mut out, band_idx, &filtered, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_non_local_means(
        input: &Raster,
        packed_rgb: bool,
        search_radius: usize,
        patch_radius: usize,
        h: f64,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let srad = search_radius.max(1) as isize;
        let prad = patch_radius.max(1) as isize;
        let h2 = (h.max(1e-6)).powi(2);
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let mut filtered = vec![nodata; rows * cols];
            filtered
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    let row = r as isize;
                    for c in 0..cols {
                        let col = c as isize;
                        let idx0 = r * cols + c;
                        let z0 = vals[idx0];
                        if z0 == nodata {
                            continue;
                        }

                        let mut wsum = 1.0;
                        let mut zsum = z0;
                        for ny in (row - srad)..=(row + srad) {
                            for nx in (col - srad)..=(col + srad) {
                                if ny == row && nx == col {
                                    continue;
                                }
                                if ny < 0 || nx < 0 || ny >= rows as isize || nx >= cols as isize {
                                    continue;
                                }
                                let zn = vals[ny as usize * cols + nx as usize];
                                if zn == nodata {
                                    continue;
                                }

                                let mut d2 = 0.0;
                                let mut pn = 0.0;
                                for py in -prad..=prad {
                                    for px in -prad..=prad {
                                        let y1 = row + py;
                                        let x1 = col + px;
                                        let y2 = ny + py;
                                        let x2 = nx + px;
                                        if y1 < 0
                                            || x1 < 0
                                            || y2 < 0
                                            || x2 < 0
                                            || y1 >= rows as isize
                                            || x1 >= cols as isize
                                            || y2 >= rows as isize
                                            || x2 >= cols as isize
                                        {
                                            continue;
                                        }
                                        let a = vals[y1 as usize * cols + x1 as usize];
                                        let b = vals[y2 as usize * cols + x2 as usize];
                                        if a == nodata || b == nodata {
                                            continue;
                                        }
                                        let d = a - b;
                                        d2 += d * d;
                                        pn += 1.0;
                                    }
                                }
                                if pn <= 0.0 {
                                    continue;
                                }
                                let w = (-(d2 / pn) / h2).exp();
                                wsum += w;
                                zsum += w * zn;
                            }
                        }
                        row_out[c] = zsum / wsum;
                    }
                });

            Self::set_band_from_values(input, &mut out, band_idx, &filtered, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_kuwahara(
        input: &Raster,
        packed_rgb: bool,
        radius: usize,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let rad = radius.max(1) as isize;
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let mut filtered = vec![nodata; rows * cols];
            filtered
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    let row = r as isize;
                    for c in 0..cols {
                        let col = c as isize;
                        let z0 = vals[r * cols + c];
                        if z0 == nodata {
                            continue;
                        }

                        let quadrants = [
                            (-rad, 0isize, -rad, 0isize),
                            (-rad, 0isize, 0isize, rad),
                            (0isize, rad, -rad, 0isize),
                            (0isize, rad, 0isize, rad),
                        ];

                        let mut best_mean = z0;
                        let mut best_var = f64::INFINITY;
                        for (dy0, dy1, dx0, dx1) in quadrants {
                            let mut n = 0.0;
                            let mut sum = 0.0;
                            let mut sum2 = 0.0;
                            for dy in dy0..=dy1 {
                                for dx in dx0..=dx1 {
                                    let y = row + dy;
                                    let x = col + dx;
                                    if y < 0 || x < 0 || y >= rows as isize || x >= cols as isize {
                                        continue;
                                    }
                                    let z = vals[y as usize * cols + x as usize];
                                    if z == nodata {
                                        continue;
                                    }
                                    n += 1.0;
                                    sum += z;
                                    sum2 += z * z;
                                }
                            }
                            if n <= 0.0 {
                                continue;
                            }
                            let mean = sum / n;
                            let var = (sum2 / n) - mean * mean;
                            if var < best_var {
                                best_var = var;
                                best_mean = mean;
                            }
                        }

                        row_out[c] = best_mean;
                    }
                });

            Self::set_band_from_values(input, &mut out, band_idx, &filtered, packed_rgb)?;
        }

        Ok(out)
    }

    fn local_mean_var(vals: &[f64], rows: usize, cols: usize, radius: usize, nodata: f64) -> (Vec<f64>, Vec<f64>) {
        let mean = Self::box_mean_from_integral(vals, rows, cols, radius, nodata);
        let sq: Vec<f64> = vals
            .iter()
            .map(|&z| if z == nodata { nodata } else { z * z })
            .collect();
        let mean_sq = Self::box_mean_from_integral(&sq, rows, cols, radius, nodata);
        let mut var = vec![nodata; rows * cols];
        for i in 0..var.len() {
            if mean[i] == nodata || mean_sq[i] == nodata {
                continue;
            }
            var[i] = (mean_sq[i] - mean[i] * mean[i]).max(0.0);
        }
        (mean, var)
    }

    fn run_frost(
        input: &Raster,
        packed_rgb: bool,
        radius: usize,
        damping_factor: f64,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let rad = radius.max(1) as isize;
        let k = damping_factor.max(1e-6);
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let (mean, var) = Self::local_mean_var(&vals, rows, cols, radius.max(1), nodata);
            let mut filtered = vec![nodata; rows * cols];
            filtered
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    let row = r as isize;
                    for c in 0..cols {
                        let col = c as isize;
                        let idx0 = r * cols + c;
                        let z0 = vals[idx0];
                        if z0 == nodata || mean[idx0] == nodata || var[idx0] == nodata {
                            continue;
                        }

                        let m = mean[idx0];
                        let v = var[idx0];
                        let alpha = k * v / (m * m + 1e-12);
                        let mut wsum = 0.0;
                        let mut zsum = 0.0;
                        for ny in (row - rad)..=(row + rad) {
                            for nx in (col - rad)..=(col + rad) {
                                if ny < 0 || nx < 0 || ny >= rows as isize || nx >= cols as isize {
                                    continue;
                                }
                                let zn = vals[ny as usize * cols + nx as usize];
                                if zn == nodata {
                                    continue;
                                }
                                let d = ((ny - row).abs() + (nx - col).abs()) as f64;
                                let w = (-alpha * d).exp();
                                wsum += w;
                                zsum += w * zn;
                            }
                        }
                        if wsum > 0.0 {
                            row_out[c] = zsum / wsum;
                        }
                    }
                });

            Self::set_band_from_values(input, &mut out, band_idx, &filtered, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_gamma_map(
        input: &Raster,
        packed_rgb: bool,
        radius: usize,
        enl: f64,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let enl = enl.max(1e-6);
        let cu = 1.0 / enl.sqrt();
        let cmax = 2.0_f64.sqrt() * cu;
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let (mean, var) = Self::local_mean_var(&vals, rows, cols, radius.max(1), nodata);
            let mut filtered = vec![nodata; rows * cols];
            filtered
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let z = vals[idx];
                        if z == nodata || mean[idx] == nodata || var[idx] == nodata {
                            continue;
                        }

                        let m = mean[idx];
                        let v = var[idx];
                        let ci = if m > 0.0 { v.sqrt() / m } else { 0.0 };
                        row_out[c] = if ci <= cu {
                            m
                        } else if ci >= cmax {
                            z
                        } else {
                            let a = ((1.0 + cu * cu) / ((ci * ci - cu * cu).max(1e-12))).max(0.0);
                            (a * m + z) / (a + 1.0)
                        };
                    }
                });

            Self::set_band_from_values(input, &mut out, band_idx, &filtered, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_kuan(
        input: &Raster,
        packed_rgb: bool,
        radius: usize,
        enl: f64,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let enl = enl.max(1e-6);
        let cu2 = 1.0 / enl;
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let (mean, var) = Self::local_mean_var(&vals, rows, cols, radius.max(1), nodata);
            let mut filtered = vec![nodata; rows * cols];
            filtered
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let z = vals[idx];
                        if z == nodata || mean[idx] == nodata || var[idx] == nodata {
                            continue;
                        }
                        let m = mean[idx];
                        let ci2 = var[idx] / (m * m + 1e-12);
                        let w = if ci2 > 0.0 {
                            ((1.0 - cu2 / ci2) / (1.0 + cu2)).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        row_out[c] = w * z + (1.0 - w) * m;
                    }
                });

            Self::set_band_from_values(input, &mut out, band_idx, &filtered, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_gabor(
        input: &Raster,
        packed_rgb: bool,
        sigma: f64,
        frequency: f64,
        orientations: usize,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let sigma = sigma.max(0.25);
        let frequency = frequency.max(1e-6);
        let orientations = orientations.max(1);
        let radius = (sigma * 3.0).ceil() as isize;
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let mut resp = vec![nodata; rows * cols];
            resp.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                let row = r as isize;
                for c in 0..cols {
                    let col = c as isize;
                    let z0 = vals[r * cols + c];
                    if z0 == nodata {
                        continue;
                    }
                    let mut best: f64 = 0.0;
                    for k in 0..orientations {
                        let theta = (k as f64) * PI / (orientations as f64);
                        let ct = theta.cos();
                        let st = theta.sin();
                        let mut sum = 0.0;
                        let mut ws = 0.0;
                        for dy in -radius..=radius {
                            for dx in -radius..=radius {
                                let y = row + dy;
                                let x = col + dx;
                                if y < 0 || x < 0 || y >= rows as isize || x >= cols as isize {
                                    continue;
                                }
                                let z = vals[y as usize * cols + x as usize];
                                if z == nodata {
                                    continue;
                                }
                                let xp = (dx as f64) * ct + (dy as f64) * st;
                                let yp = -(dx as f64) * st + (dy as f64) * ct;
                                let g = (-(xp * xp + yp * yp) / (2.0 * sigma * sigma)).exp();
                                let w = g * (2.0 * PI * frequency * xp).cos();
                                sum += w * z;
                                ws += w.abs();
                            }
                        }
                        if ws > 0.0 {
                            best = best.max((sum / ws).abs());
                        }
                    }
                    out_row[c] = best;
                }
            });

            Self::set_band_from_values(input, &mut out, band_idx, &resp, false)?;
        }

        Ok(out)
    }

    fn run_frangi(
        input: &Raster,
        packed_rgb: bool,
        scales: &[f64],
        beta: f64,
        c: f64,
    ) -> Result<Raster, ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let beta2 = 2.0 * beta.max(1e-6) * beta.max(1e-6);
        let c2 = 2.0 * c.max(1e-6) * c.max(1e-6);
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c0 in 0..cols {
                    let z_raw = input.get(band, r as isize, c0 as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c0] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let mut vessel = vec![nodata; rows * cols];
            vessel.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                for c0 in 0..cols {
                    let idx = r * cols + c0;
                    if vals[idx] == nodata {
                        continue;
                    }
                    let mut best = 0.0;
                    for &s in scales {
                        let rad = s.max(1.0).round() as isize;
                        let mut sx = 0.0;
                        let mut sy = 0.0;
                        let mut n = 0.0;
                        for dy in -rad..=rad {
                            for dx in -rad..=rad {
                                let y = r as isize + dy;
                                let x = c0 as isize + dx;
                                if y < 0 || x < 0 || y >= rows as isize || x >= cols as isize {
                                    continue;
                                }
                                let z = vals[y as usize * cols + x as usize];
                                if z == nodata {
                                    continue;
                                }
                                sx += z * (dx as f64);
                                sy += z * (dy as f64);
                                n += 1.0;
                            }
                        }
                        if n <= 0.0 {
                            continue;
                        }
                        let ix = sx / n;
                        let iy = sy / n;
                        let ixx = ix * ix;
                        let iyy = iy * iy;
                        let ixy = ix * iy;

                        let tr = ixx + iyy;
                        let det_term = ((ixx - iyy) * (ixx - iyy) + 4.0 * ixy * ixy).sqrt();
                        let l1 = 0.5 * (tr + det_term);
                        let l2 = 0.5 * (tr - det_term);
                        let (lam1, lam2) = if l1.abs() <= l2.abs() { (l1, l2) } else { (l2, l1) };

                        if lam2 >= 0.0 {
                            continue;
                        }
                        let rb = (lam1 / (lam2 + 1e-12)).powi(2);
                        let s2 = lam1 * lam1 + lam2 * lam2;
                        let v = (-rb / beta2).exp() * (1.0 - (-s2 / c2).exp());
                        if v > best {
                            best = v;
                        }
                    }
                    out_row[c0] = best;
                }
            });

            Self::set_band_from_values(input, &mut out, band_idx, &vessel, false)?;
        }

        Ok(out)
    }

    fn run_savgol2d(
        input: &Raster,
        packed_rgb: bool,
        window_size: usize,
    ) -> Result<Raster, ToolError> {
        let ws = if window_size % 2 == 1 { window_size } else { window_size + 1 };
        if ws != 5 {
            return Err(ToolError::Validation(
                "savitzky_golay_2d_filter currently supports window_size=5 only".to_string(),
            ));
        }

        let kernel: [[f64; 5]; 5] = [
            [-3.0, 12.0, 17.0, 12.0, -3.0],
            [12.0, 2.0, -3.0, 2.0, 12.0],
            [17.0, -3.0, -12.0, -3.0, 17.0],
            [12.0, 2.0, -3.0, 2.0, 12.0],
            [-3.0, 12.0, 17.0, 12.0, -3.0],
        ];
        let norm = 35.0;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let mut out = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let z_raw = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z_raw) {
                        vals[r * cols + c] = if packed_rgb { value2i(z_raw) } else { z_raw };
                    }
                }
            }

            let mut filtered = vec![nodata; rows * cols];
            filtered.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                let row = r as isize;
                for c in 0..cols {
                    let col = c as isize;
                    let z0 = vals[r * cols + c];
                    if z0 == nodata {
                        continue;
                    }
                    let mut sum = 0.0;
                    let mut wsum = 0.0;
                    for ky in 0..5 {
                        for kx in 0..5 {
                            let y = row + ky as isize - 2;
                            let x = col + kx as isize - 2;
                            if y < 0 || x < 0 || y >= rows as isize || x >= cols as isize {
                                continue;
                            }
                            let z = vals[y as usize * cols + x as usize];
                            if z == nodata {
                                continue;
                            }
                            let w = kernel[ky][kx] / norm;
                            sum += w * z;
                            wsum += w;
                        }
                    }
                    if wsum.abs() > 1e-12 {
                        out_row[c] = sum / wsum;
                    } else {
                        out_row[c] = z0;
                    }
                }
            });

            Self::set_band_from_values(input, &mut out, band_idx, &filtered, packed_rgb)?;
        }

        Ok(out)
    }

    fn run_with_op(op: AdvancedOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info(&format!("running {}", op.id()));
        let input = Self::load_raster(&input_path)?;
        let rgb_mode = color_support::detect_rgb_mode(&input, false, true);
        let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && input.bands == 1;

        let output = match op {
            AdvancedOp::AnisotropicDiffusion => {
                let iterations = args.get("iterations").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let kappa = args.get("kappa").and_then(|v| v.as_f64()).unwrap_or(20.0);
                let lambda = args.get("lambda").and_then(|v| v.as_f64()).unwrap_or(0.2);
                Self::run_anisotropic(&input, packed_rgb, iterations.max(1), kappa, lambda)?
            }
            AdvancedOp::GammaCorrection => {
                let gamma = args.get("gamma").and_then(|v| v.as_f64()).unwrap_or(0.5);
                Self::run_gamma(&input, packed_rgb, gamma)?
            }
            AdvancedOp::Guided => {
                let radius = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(4) as usize;
                let epsilon = args.get("epsilon").and_then(|v| v.as_f64()).unwrap_or(0.01);
                Self::run_guided(&input, packed_rgb, radius, epsilon)?
            }
            AdvancedOp::Wiener => {
                let radius = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                let noise_variance = args.get("noise_variance").and_then(|v| v.as_f64());
                Self::run_wiener(&input, packed_rgb, radius, noise_variance)?
            }
            AdvancedOp::NonLocalMeans => {
                let search_radius = args
                    .get("search_radius")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as usize;
                let patch_radius = args
                    .get("patch_radius")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;
                let h = args.get("h").and_then(|v| v.as_f64()).unwrap_or(10.0);
                Self::run_non_local_means(&input, packed_rgb, search_radius, patch_radius, h)?
            }
            AdvancedOp::Kuwahara => {
                let radius = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                Self::run_kuwahara(&input, packed_rgb, radius)?
            }
            AdvancedOp::Frost => {
                let radius = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                let damping_factor = args
                    .get("damping_factor")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(2.0);
                Self::run_frost(&input, packed_rgb, radius, damping_factor)?
            }
            AdvancedOp::GammaMap => {
                let radius = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                let enl = args.get("enl").and_then(|v| v.as_f64()).unwrap_or(1.0);
                Self::run_gamma_map(&input, packed_rgb, radius, enl)?
            }
            AdvancedOp::Kuan => {
                let radius = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                let enl = args.get("enl").and_then(|v| v.as_f64()).unwrap_or(1.0);
                Self::run_kuan(&input, packed_rgb, radius, enl)?
            }
            AdvancedOp::Gabor => {
                let sigma = args.get("sigma").and_then(|v| v.as_f64()).unwrap_or(2.0);
                let frequency = args
                    .get("frequency")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.2);
                let orientations = args
                    .get("orientations")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(6) as usize;
                Self::run_gabor(&input, packed_rgb, sigma, frequency, orientations)?
            }
            AdvancedOp::Frangi => {
                let scales = args
                    .get("scales")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![1.0, 2.0, 3.0]);
                let beta = args.get("beta").and_then(|v| v.as_f64()).unwrap_or(0.5);
                let c = args.get("c").and_then(|v| v.as_f64()).unwrap_or(15.0);
                Self::run_frangi(&input, packed_rgb, &scales, beta, c)?
            }
            AdvancedOp::SavitzkyGolay2d => {
                let window_size = args
                    .get("window_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as usize;
                Self::run_savgol2d(&input, packed_rgb, window_size)?
            }
        };

        ctx.progress.progress(1.0);
        let output_locator = Self::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

struct AdvancedFilters;

macro_rules! define_adv_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                AdvancedFilters::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                AdvancedFilters::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = AdvancedFilters::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                AdvancedFilters::run_with_op($op, args, ctx)
            }
        }
    };
}

define_adv_tool!(AnisotropicDiffusionFilterTool, AdvancedOp::AnisotropicDiffusion);
define_adv_tool!(GammaCorrectionTool, AdvancedOp::GammaCorrection);
define_adv_tool!(GuidedFilterTool, AdvancedOp::Guided);
define_adv_tool!(WienerFilterTool, AdvancedOp::Wiener);
define_adv_tool!(NonLocalMeansFilterTool, AdvancedOp::NonLocalMeans);
define_adv_tool!(KuwaharaFilterTool, AdvancedOp::Kuwahara);
define_adv_tool!(FrostFilterTool, AdvancedOp::Frost);
define_adv_tool!(GammaMapFilterTool, AdvancedOp::GammaMap);
define_adv_tool!(KuanFilterTool, AdvancedOp::Kuan);
define_adv_tool!(GaborFilterBankTool, AdvancedOp::Gabor);
define_adv_tool!(FrangiFilterTool, AdvancedOp::Frangi);
define_adv_tool!(SavitzkyGolay2dFilterTool, AdvancedOp::SavitzkyGolay2d);

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
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                r.set(0, row, col, value).unwrap();
            }
        }
        r
    }

    fn run_with_memory(tool: &dyn Tool, args: &mut ToolArgs, input: Raster) -> Raster {
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);
        args.insert("input".to_string(), json!(input_path));
        let result = tool.run(args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    #[test]
    fn anisotropic_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("iterations".to_string(), json!(10));
        args.insert("kappa".to_string(), json!(20.0));
        args.insert("lambda".to_string(), json!(0.2));
        let out = run_with_memory(
            &AnisotropicDiffusionFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn gamma_unit_input_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("gamma".to_string(), json!(0.5));
        let out = run_with_memory(&GammaCorrectionTool, &mut args, make_constant_raster(25, 25, 1.0));
        assert!((out.get(0, 12, 12) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn guided_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("radius".to_string(), json!(4));
        args.insert("epsilon".to_string(), json!(0.01));
        let out = run_with_memory(&GuidedFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn wiener_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("radius".to_string(), json!(2));
        let out = run_with_memory(&WienerFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn nlm_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("search_radius".to_string(), json!(3));
        args.insert("patch_radius".to_string(), json!(1));
        args.insert("h".to_string(), json!(10.0));
        let out = run_with_memory(
            &NonLocalMeansFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn kuwahara_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("radius".to_string(), json!(2));
        let out = run_with_memory(&KuwaharaFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn frost_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("radius".to_string(), json!(2));
        args.insert("damping_factor".to_string(), json!(2.0));
        let out = run_with_memory(&FrostFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn gamma_map_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("radius".to_string(), json!(2));
        args.insert("enl".to_string(), json!(1.0));
        let out = run_with_memory(&GammaMapFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn kuan_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("radius".to_string(), json!(2));
        args.insert("enl".to_string(), json!(1.0));
        let out = run_with_memory(&KuanFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn gabor_constant_raster_non_negative() {
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(2.0));
        args.insert("frequency".to_string(), json!(0.2));
        args.insert("orientations".to_string(), json!(6));
        let out = run_with_memory(&GaborFilterBankTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!(out.get(0, 12, 12) >= 0.0);
    }

    #[test]
    fn frangi_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("scales".to_string(), json!([1.0, 2.0, 3.0]));
        args.insert("beta".to_string(), json!(0.5));
        args.insert("c".to_string(), json!(15.0));
        let out = run_with_memory(&FrangiFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }

    #[test]
    fn savgol_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("window_size".to_string(), json!(5));
        let out = run_with_memory(
            &SavitzkyGolay2dFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }
}
