use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::sync::Arc;

use super::color_support;
use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::color_math::{hsi2value, hsi_to_rgb_norm, rgb_to_hsi_norm, value2hsi, value2i};
use wbraster::{DataType, Raster, RasterFormat};

use crate::memory_store;

pub struct GaussianFilterTool;

impl GaussianFilterTool {
    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation(
                    "parameter 'input' has malformed in-memory raster path".to_string(),
                )
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

    fn build_kernel(sigma: f64) -> (Vec<isize>, Vec<isize>, Vec<f64>, usize) {
        let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * PI).sqrt() * sigma);
        let two_sigma_sqr_d = 2.0 * sigma * sigma;

        let mut filter_size = 0usize;
        for i in 0..250usize {
            let dist2 = (i * i) as f64;
            let weight = recip_root_2_pi_times_sigma_d * (-dist2 / two_sigma_sqr_d).exp();
            if weight <= 0.001 {
                filter_size = i * 2 + 1;
                break;
            }
        }
        if filter_size % 2 == 0 {
            filter_size += 1;
        }
        if filter_size < 3 {
            filter_size = 3;
        }

        let num_filter = filter_size * filter_size;
        let midpoint = (filter_size as f64 / 2.0).floor() as isize;

        let mut dx = vec![0isize; num_filter];
        let mut dy = vec![0isize; num_filter];
        let mut weights = vec![0.0f64; num_filter];

        let mut a = 0;
        let mut weight_sum = 0.0;
        for row in 0..filter_size {
            for col in 0..filter_size {
                let x = col as isize - midpoint;
                let y = row as isize - midpoint;
                dx[a] = x;
                dy[a] = y;
                let w = recip_root_2_pi_times_sigma_d * (-(x * x + y * y) as f64 / two_sigma_sqr_d).exp();
                weights[a] = w;
                weight_sum += w;
                a += 1;
            }
        }

        if weight_sum > 0.0 {
            for w in &mut weights {
                *w /= weight_sum;
            }
        }

        (dx, dy, weights, num_filter)
    }
}

impl Tool for GaussianFilterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "gaussian_filter",
            display_name: "Gaussian Filter",
            summary: "Performs Gaussian smoothing on a raster image.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "sigma",
                    description: "Gaussian standard deviation in pixels (0.5-20.0, default 0.75).",
                    required: false,
                },
                ToolParamSpec {
                    name: "treat_as_rgb",
                    description: "Set true to force RGB intensity-space behavior.",
                    required: false,
                },
                ToolParamSpec {
                    name: "assume_three_band_rgb",
                    description: "When true (default), 3-band uint8/uint16 rasters may be treated as RGB if explicit metadata is missing.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output file path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("sigma".to_string(), json!(0.75));
        defaults.insert("treat_as_rgb".to_string(), json!(false));
        defaults.insert("assume_three_band_rgb".to_string(), json!(true));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("sigma".to_string(), json!(1.5));
        example_args.insert("output".to_string(), json!("image_gaussian.tif"));

        ToolManifest {
            id: "gaussian_filter".to_string(),
            display_name: "Gaussian Filter".to_string(),
            summary: "Performs Gaussian smoothing on a raster image.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "sigma".to_string(),
                    description: "Gaussian standard deviation in pixels (0.5-20.0, default 0.75).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "treat_as_rgb".to_string(),
                    description: "Set true to force RGB intensity-space behavior.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "assume_three_band_rgb".to_string(),
                    description: "When true (default), 3-band uint8/uint16 rasters may be treated as RGB if explicit metadata is missing.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_gaussian_filter".to_string(),
                description: "Applies Gaussian smoothing to a raster.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "image".to_string(),
                "filter".to_string(),
                "smoothing".to_string(),
                "gaussian".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;

        let sigma = args
            .get("sigma")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75)
            .clamp(0.5, 20.0);

        let treat_as_rgb_requested = args
            .get("treat_as_rgb")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let assume_three_band_rgb = args
            .get("assume_three_band_rgb")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        ctx.progress.info("running gaussian_filter");
        ctx.progress.info("reading input raster");

        let input = Self::load_raster(&input_path)?;
        let rgb_mode = color_support::detect_rgb_mode(
            &input,
            treat_as_rgb_requested,
            assume_three_band_rgb,
        );

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let (dx, dy, weights, num_filter) = Self::build_kernel(sigma);

        let input = Arc::new(input);
        let dx = Arc::new(dx);
        let dy = Arc::new(dy);
        let weights = Arc::new(weights);

        ctx.progress.info("applying gaussian filter");

        let mut output = input.as_ref().clone();

        if matches!(rgb_mode, color_support::RgbMode::ThreeBand) && bands >= 3 {
            let max_val = if input.data_type == DataType::U8 { 255.0 } else { 65535.0 };
            let inp = input.as_ref();
            let dxv: &[isize] = &dx;
            let dyv: &[isize] = &dy;
            let wv: &[f64] = &weights;

            let mut out_rgb = vec![[nodata; 3]; rows * cols];
            out_rgb
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(row_idx, out_row)| {
                    let row = row_idx as isize;
                    for col_idx in 0..cols {
                        let col = col_idx as isize;

                        let r0 = inp.get(0, row, col);
                        let g0 = inp.get(1, row, col);
                        let b0 = inp.get(2, row, col);
                        if inp.is_nodata(r0) || inp.is_nodata(g0) || inp.is_nodata(b0) {
                            continue;
                        }

                        let rn0 = (r0 / max_val).clamp(0.0, 1.0);
                        let gn0 = (g0 / max_val).clamp(0.0, 1.0);
                        let bn0 = (b0 / max_val).clamp(0.0, 1.0);
                        let mut sum = 0.0;
                        let mut z_final = 0.0;
                        for a in 0..num_filter {
                            let nx = col + dxv[a];
                            let ny = row + dyv[a];
                            let r = inp.get(0, ny, nx);
                            let g = inp.get(1, ny, nx);
                            let b = inp.get(2, ny, nx);
                            if inp.is_nodata(r) || inp.is_nodata(g) || inp.is_nodata(b) {
                                continue;
                            }
                            let rn = (r / max_val).clamp(0.0, 1.0);
                            let gn = (g / max_val).clamp(0.0, 1.0);
                            let bn = (b / max_val).clamp(0.0, 1.0);
                            let zn = (rn + gn + bn) / 3.0;
                            sum += wv[a];
                            z_final += wv[a] * zn;
                        }

                        if sum > 0.0 {
                            let (h, s, _) = rgb_to_hsi_norm(rn0, gn0, bn0);
                            let (ro, go, bo) = hsi_to_rgb_norm(h, s, (z_final / sum).clamp(0.0, 1.0));
                            out_row[col_idx] = [ro * max_val, go * max_val, bo * max_val];
                        }
                    }
                });

            for row_idx in 0..rows {
                let mut row_r = vec![nodata; cols];
                let mut row_g = vec![nodata; cols];
                let mut row_b = vec![nodata; cols];
                for col_idx in 0..cols {
                    let px = out_rgb[row_idx * cols + col_idx];
                    row_r[col_idx] = px[0];
                    row_g[col_idx] = px[1];
                    row_b[col_idx] = px[2];
                }
                output
                    .set_row_slice(0, row_idx as isize, &row_r)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {} band 0: {}", row_idx, e)))?;
                output
                    .set_row_slice(1, row_idx as isize, &row_g)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {} band 1: {}", row_idx, e)))?;
                output
                    .set_row_slice(2, row_idx as isize, &row_b)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {} band 2: {}", row_idx, e)))?;
            }
        } else {
            let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && bands == 1;

            for band_idx in 0..bands {
                let band = band_idx as isize;
                let mut out_data = vec![nodata; rows * cols];

                let inp = input.as_ref();
                let dxv: &[isize] = &dx;
                let dyv: &[isize] = &dy;
                let wv: &[f64] = &weights;

                out_data
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(row_idx, out_row)| {
                        let row = row_idx as isize;

                        for col_idx in 0..cols {
                            let col = col_idx as isize;
                            let z_raw = inp.get(band, row, col);
                            if inp.is_nodata(z_raw) {
                                continue;
                            }

                            let mut sum = 0.0;
                            let mut z_final = 0.0;

                            for a in 0..num_filter {
                                let nx = col + dxv[a];
                                let ny = row + dyv[a];
                                let zn_raw = inp.get(band, ny, nx);
                                if inp.is_nodata(zn_raw) {
                                    continue;
                                }
                                let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                sum += wv[a];
                                z_final += wv[a] * zn;
                            }

                            if sum > 0.0 {
                                let filtered = z_final / sum;
                                out_row[col_idx] = if packed_rgb {
                                    let (h, s, _) = value2hsi(z_raw);
                                    hsi2value(h, s, filtered)
                                } else {
                                    filtered
                                };
                            }
                        }
                    });

                for row_idx in 0..rows {
                    let start = row_idx * cols;
                    output
                        .set_row_slice(band, row_idx as isize, &out_data[start..start + cols])
                        .map_err(|e| {
                            ToolError::Execution(format!("failed writing row {}: {}", row_idx, e))
                        })?;
                }

                ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
            }
        }

        let output_locator = if let Some(output_path) = output_path {
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

    #[test]
    fn gaussian_filter_constant_raster_is_unchanged() {
        let input = make_constant_raster(20, 20, 42.0);
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));
        args.insert("sigma".to_string(), json!(1.0));

        let result = GaussianFilterTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        assert!(out_path.starts_with("memory://raster/"));

        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        let out_raster = memory_store::get_raster_by_id(out_id).unwrap();
        for row in 0..20isize {
            for col in 0..20isize {
                let v = out_raster.get(0, row, col);
                assert!((v - 42.0).abs() < 1e-9);
            }
        }
    }
}
