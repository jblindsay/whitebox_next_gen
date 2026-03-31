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
use wbraster::color_math::{hsi2value, value2hsi, value2i};
use wbraster::{Raster, RasterFormat};
use crate::memory_store;

pub struct BilateralFilterTool;
pub struct HighPassBilateralFilterTool;

impl BilateralFilterTool {
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

    /// Build the distance-domain lookup tables (dx, dy, weights_d) for the given sigma_dist.
    ///
    /// The filter radius is determined by stepping outward until the spatial Gaussian weight
    /// drops to ≤ 0.001, matching the legacy WbW behaviour.
    fn build_kernel(sigma_dist: f64) -> (Vec<isize>, Vec<isize>, Vec<f64>, usize) {
        let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * PI).sqrt() * sigma_dist);
        let two_sigma_sqr_d = 2.0 * sigma_dist * sigma_dist;

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
        let mut weights_d = vec![0.0f64; num_filter];

        let mut a = 0;
        for row in 0..filter_size {
            for col in 0..filter_size {
                let x = col as isize - midpoint;
                let y = row as isize - midpoint;
                dx[a] = x;
                dy[a] = y;
                weights_d[a] = recip_root_2_pi_times_sigma_d
                    * (-(x * x + y * y) as f64 / two_sigma_sqr_d).exp();
                a += 1;
            }
        }

        (dx, dy, weights_d, num_filter)
    }
}

impl Tool for BilateralFilterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "bilateral_filter",
            display_name: "Bilateral Filter",
            summary: "Performs an edge-preserving bilateral smoothing filter on a raster image.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "sigma_dist",
                    description: "Standard deviation of the spatial (distance) Gaussian kernel, \
                                  in pixels (0.5–20.0, default 0.75). Controls the filter radius.",
                    required: false,
                },
                ToolParamSpec {
                    name: "sigma_int",
                    description: "Standard deviation of the intensity Gaussian kernel, in the \
                                  same units as the raster values (default 1.0). Controls \
                                  edge-preservation strength.",
                    required: false,
                },
                ToolParamSpec {
                    name: "treat_as_rgb",
                    description: "Set true to force HSI-intensity bilateral filtering for packed \
                                  RGB rasters. When false, packed RGB is still auto-detected from \
                                  standardized raster metadata when available.",
                    required: false,
                },
                ToolParamSpec {
                    name: "assume_three_band_rgb",
                    description: "When true (default), and no explicit color metadata is present, \
                                  allow 3-band uint8/uint16 RGB interpretation in RGB-capable filters. \
                                  Disable for multispectral 3-band datasets.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output file path. If omitted, output remains in memory \
                                  and is returned as a memory:// raster handle.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("sigma_dist".to_string(), json!(0.75));
        defaults.insert("sigma_int".to_string(), json!(1.0));
        defaults.insert("treat_as_rgb".to_string(), json!(false));
        defaults.insert("assume_three_band_rgb".to_string(), json!(true));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("sigma_dist".to_string(), json!(1.5));
        example_args.insert("sigma_int".to_string(), json!(25.0));
        example_args.insert("treat_as_rgb".to_string(), json!(false));
        example_args.insert("assume_three_band_rgb".to_string(), json!(true));
        example_args.insert("output".to_string(), json!("image_bilateral.tif"));

        ToolManifest {
            id: "bilateral_filter".to_string(),
            display_name: "Bilateral Filter".to_string(),
            summary: "Performs an edge-preserving bilateral smoothing filter on a raster image."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "sigma_dist".to_string(),
                    description: "Standard deviation of the spatial Gaussian kernel in pixels \
                                  (0.5–20.0, default 0.75)."
                        .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "sigma_int".to_string(),
                    description: "Standard deviation of the intensity Gaussian kernel in raster \
                                  value units (default 1.0)."
                        .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "treat_as_rgb".to_string(),
                    description: "Set true to force packed RGB HSI processing. When false, packed \
                                  RGB may still be auto-detected from raster metadata."
                        .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "assume_three_band_rgb".to_string(),
                    description: "When true (default), 3-band uint8/uint16 rasters may be treated \
                                  as RGB in filters that support 3-band RGB intensity processing."
                        .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory."
                        .to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_bilateral_filter".to_string(),
                description: "Applies bilateral filter to an image.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "image".to_string(),
                "filter".to_string(),
                "smoothing".to_string(),
                "edge-preserving".to_string(),
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

        // Parse sigma_dist; clamp to the range that produces a valid kernel.
        let sigma_dist = args
            .get("sigma_dist")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75)
            .clamp(0.5, 20.0);

        // Parse sigma_int; guard against near-zero to avoid numerical blow-up.
        let sigma_int = {
            let v = args
                .get("sigma_int")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            if v < 0.001 { 0.001 } else { v }
        };

        let treat_as_rgb_requested = args
            .get("treat_as_rgb")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let assume_three_band_rgb = args
            .get("assume_three_band_rgb")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        ctx.progress.info("running bilateral_filter");
        ctx.progress.info("reading input raster");

        let input = Self::load_raster(&input_path)?;

        // Prefer explicit metadata-based interpretation and optional caller override,
        // with an optional 3-band heuristic fallback for common RGB imagery.
        let rgb_mode = color_support::detect_rgb_mode(
            &input,
            treat_as_rgb_requested,
            assume_three_band_rgb,
        );
        let treat_as_rgb = matches!(rgb_mode, color_support::RgbMode::Packed);

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        // Precompute intensity-domain Gaussian constants (Copy scalars; captured cheaply).
        let recip_root_2_pi_times_sigma_i = 1.0 / ((2.0 * PI).sqrt() * sigma_int);
        let two_sigma_sqr_i = 2.0 * sigma_int * sigma_int;

        // Build spatial kernel lookup tables.
        let (dx, dy, weights_d, num_filter) = Self::build_kernel(sigma_dist);

        let input = Arc::new(input);
        let dx = Arc::new(dx);
        let dy = Arc::new(dy);
        let weights_d = Arc::new(weights_d);

        ctx.progress.info("applying bilateral filter");

        // Clone the full raster so output metadata (georeference, nodata, etc.) is preserved.
        let mut output = input.as_ref().clone();

        // Process each band independently.  For true per-band operation this is correct; a
        // future HSI-aware path can be layered on top without changing this structure.
        for band_idx in 0..bands {
            let band = band_idx as isize;

            // Allocate a flat row-major output buffer for this band, pre-filled with nodata.
            let mut out_data = vec![nodata; rows * cols];

            // Borrow shared data as plain references so the parallel closure is Send + Sync
            // without moving the Arcs.
            let inp = input.as_ref();
            let dxv: &[isize] = &dx;
            let dyv: &[isize] = &dy;
            let wdv: &[f64] = &weights_d;

            // Split the output buffer into row-sized slices and process each row in parallel.
            out_data
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(row_idx, out_row)| {
                    let row = row_idx as isize;

                    // Per-row scratch buffers reused for every column.
                    let mut weights_i = vec![0.0f64; num_filter];
                    let mut neighbor_vals = vec![nodata; num_filter];

                    for col_idx in 0..cols {
                        let col = col_idx as isize;
                        let z_raw = inp.get(band, row, col);
                        let z = if treat_as_rgb { value2i(z_raw) } else { z_raw };
                        if inp.is_nodata(z_raw) {
                            // out_row[col_idx] is already nodata; nothing to do.
                            continue;
                        }

                        // ------------------------------------------------------------------
                        // Pass 1: read all neighbours once, compute combined bilateral weight.
                        // The accessor returns nodata (and is marked as such) for OOB coords,
                        // treating the image boundary identically to interior nodata cells.
                        // ------------------------------------------------------------------
                        let mut sum = 0.0f64;
                        for a in 0..num_filter {
                            let nx = col + dxv[a];
                            let ny = row + dyv[a];
                            let zn_raw = inp.get(band, ny, nx);
                            let zn = if treat_as_rgb { value2i(zn_raw) } else { zn_raw };
                            neighbor_vals[a] = zn;
                            if !inp.is_nodata(zn_raw) {
                                let diff = zn - z;
                                let wi = recip_root_2_pi_times_sigma_i
                                    * (-diff * diff / two_sigma_sqr_i).exp();
                                let w = wi * wdv[a];
                                weights_i[a] = w;
                                sum += w;
                            } else {
                                weights_i[a] = 0.0;
                            }
                        }

                        // ------------------------------------------------------------------
                        // Pass 2: normalised weighted sum using cached neighbour values.
                        // ------------------------------------------------------------------
                        if sum > 0.0 {
                            let mut z_final = 0.0f64;
                            for a in 0..num_filter {
                                if weights_i[a] > 0.0 {
                                    z_final += weights_i[a] * neighbor_vals[a] / sum;
                                }
                            }
                            out_row[col_idx] = if treat_as_rgb {
                                let (h, s, _) = value2hsi(z_raw);
                                hsi2value(h, s, z_final)
                            } else {
                                z_final
                            };
                        }
                    }
                });

            // Write the processed band back into the output raster.
            for row_idx in 0..rows {
                let start = row_idx * cols;
                output
                    .set_row_slice(band, row_idx as isize, &out_data[start..start + cols])
                    .map_err(|e| {
                        ToolError::Execution(format!("failed writing row {}: {}", row_idx, e))
                    })?;
            }

            ctx.progress
                .progress((band_idx + 1) as f64 / bands as f64);
        }

        // ------------------------------------------------------------------
        // Persist or store in memory.
        // ------------------------------------------------------------------
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

impl Tool for HighPassBilateralFilterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "high_pass_bilateral_filter",
            display_name: "High-Pass Bilateral Filter",
            summary: "Computes a high-pass residual by subtracting bilateral smoothing from the input raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "sigma_dist",
                    description: "Standard deviation of the spatial (distance) Gaussian kernel, in pixels (0.5–20.0, default 0.75).",
                    required: false,
                },
                ToolParamSpec {
                    name: "sigma_int",
                    description: "Standard deviation of the intensity Gaussian kernel, in raster-value units (default 1.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "treat_as_rgb",
                    description: "Set true to force HSI-intensity filtering for packed RGB rasters before high-pass differencing.",
                    required: false,
                },
                ToolParamSpec {
                    name: "assume_three_band_rgb",
                    description: "When true (default), and no explicit color metadata is present, allow 3-band uint8/uint16 RGB interpretation.",
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
        defaults.insert("sigma_dist".to_string(), json!(0.75));
        defaults.insert("sigma_int".to_string(), json!(1.0));
        defaults.insert("treat_as_rgb".to_string(), json!(false));
        defaults.insert("assume_three_band_rgb".to_string(), json!(true));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("sigma_dist".to_string(), json!(1.5));
        example_args.insert("sigma_int".to_string(), json!(25.0));
        example_args.insert("treat_as_rgb".to_string(), json!(false));
        example_args.insert("assume_three_band_rgb".to_string(), json!(true));
        example_args.insert("output".to_string(), json!("image_highpass_bilateral.tif"));

        ToolManifest {
            id: "high_pass_bilateral_filter".to_string(),
            display_name: "High-Pass Bilateral Filter".to_string(),
            summary: "Computes a high-pass residual by subtracting bilateral smoothing from the input raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: self
                .metadata()
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_high_pass_bilateral_filter".to_string(),
                description: "Applies high-pass bilateral filtering to emphasize local texture.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "image".to_string(),
                "filter".to_string(),
                "high-pass".to_string(),
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

        let sigma_dist = args
            .get("sigma_dist")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75)
            .clamp(0.5, 20.0);

        let sigma_int = {
            let v = args
                .get("sigma_int")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            if v < 0.001 { 0.001 } else { v }
        };

        let treat_as_rgb_requested = args
            .get("treat_as_rgb")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let assume_three_band_rgb = args
            .get("assume_three_band_rgb")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        ctx.progress.info("running high_pass_bilateral_filter");
        ctx.progress.info("reading input raster");

        let input = BilateralFilterTool::load_raster(&input_path)?;

        let rgb_mode = color_support::detect_rgb_mode(
            &input,
            treat_as_rgb_requested,
            assume_three_band_rgb,
        );
        let treat_as_rgb = matches!(rgb_mode, color_support::RgbMode::Packed);

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let recip_root_2_pi_times_sigma_i = 1.0 / ((2.0 * PI).sqrt() * sigma_int);
        let two_sigma_sqr_i = 2.0 * sigma_int * sigma_int;

        let (dx, dy, weights_d, num_filter) = BilateralFilterTool::build_kernel(sigma_dist);

        let input = Arc::new(input);
        let dx = Arc::new(dx);
        let dy = Arc::new(dy);
        let weights_d = Arc::new(weights_d);

        ctx.progress.info("applying high-pass bilateral filter");

        let mut output = input.as_ref().clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut out_data = vec![nodata; rows * cols];

            let inp = input.as_ref();
            let dxv: &[isize] = &dx;
            let dyv: &[isize] = &dy;
            let wdv: &[f64] = &weights_d;

            out_data
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(row_idx, out_row)| {
                    let row = row_idx as isize;
                    let mut weights_i = vec![0.0f64; num_filter];
                    let mut neighbor_vals = vec![nodata; num_filter];

                    for col_idx in 0..cols {
                        let col = col_idx as isize;
                        let z_raw = inp.get(band, row, col);
                        let z = if treat_as_rgb { value2i(z_raw) } else { z_raw };
                        if inp.is_nodata(z_raw) {
                            continue;
                        }

                        let mut sum = 0.0f64;
                        for a in 0..num_filter {
                            let nx = col + dxv[a];
                            let ny = row + dyv[a];
                            let zn_raw = inp.get(band, ny, nx);
                            let zn = if treat_as_rgb { value2i(zn_raw) } else { zn_raw };
                            neighbor_vals[a] = zn;
                            if !inp.is_nodata(zn_raw) {
                                let diff = zn - z;
                                let wi = recip_root_2_pi_times_sigma_i
                                    * (-diff * diff / two_sigma_sqr_i).exp();
                                let w = wi * wdv[a];
                                weights_i[a] = w;
                                sum += w;
                            } else {
                                weights_i[a] = 0.0;
                            }
                        }

                        if sum > 0.0 {
                            let mut z_smooth = 0.0f64;
                            for a in 0..num_filter {
                                if weights_i[a] > 0.0 {
                                    z_smooth += weights_i[a] * neighbor_vals[a] / sum;
                                }
                            }
                            out_row[col_idx] = z - z_smooth;
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

            ctx.progress
                .progress((band_idx + 1) as f64 / bands as f64);
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

    /// Build a small synthetic raster filled with a constant value.
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
    fn bilateral_filter_constant_raster_is_unchanged() {
        // A flat raster should pass through the bilateral filter unchanged — every
        // neighbour has the same value, so the intensity weight is maximised for
        // every pixel and the weighted average equals the original value.
        let input = make_constant_raster(20, 20, 42.0);
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));
        args.insert("sigma_dist".to_string(), json!(1.0));
        args.insert("sigma_int".to_string(), json!(10.0));

        let result = BilateralFilterTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        assert!(out_path.starts_with("memory://raster/"));

        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        let out_raster = memory_store::get_raster_by_id(out_id).unwrap();
        for row in 0..20isize {
            for col in 0..20isize {
                let v = out_raster.get(0, row, col);
                assert!(
                    (v - 42.0).abs() < 1e-9,
                    "expected 42.0 at ({},{}) but got {}", row, col, v
                );
            }
        }
    }

    #[test]
    fn bilateral_filter_preserves_memory_path_output() {
        // When no output path is given the tool should return a memory:// locator.
        let input = make_constant_raster(10, 10, 1.0);
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));

        let result = BilateralFilterTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(out_path.starts_with("memory://raster/"), "expected memory path, got {}", out_path);
    }

    #[test]
    fn high_pass_bilateral_filter_constant_raster_is_near_zero() {
        let input = make_constant_raster(20, 20, 42.0);
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));
        args.insert("sigma_dist".to_string(), json!(1.0));
        args.insert("sigma_int".to_string(), json!(10.0));

        let result = HighPassBilateralFilterTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        assert!(out_path.starts_with("memory://raster/"));

        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        let out_raster = memory_store::get_raster_by_id(out_id).unwrap();
        for row in 0..20isize {
            for col in 0..20isize {
                let v = out_raster.get(0, row, col);
                assert!(v.abs() < 1e-7, "expected near-zero high-pass value at ({},{}) but got {}", row, col, v);
            }
        }
    }
}
