use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::sync::Arc;

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

pub struct FastAlmostGaussianFilterTool;
pub struct EdgePreservingMeanFilterTool;
pub struct UnsharpMaskingTool;
pub struct DiffOfGaussiansFilterTool;
pub struct AdaptiveFilterTool;
pub struct LeeFilterTool;
pub struct RefinedLeeFilterTool;
pub struct EnhancedLeeFilterTool;
pub struct ConservativeSmoothingFilterTool;
pub struct OlympicFilterTool;
pub struct KNearestMeanFilterTool;
pub struct HighPassMedianFilterTool;
pub struct LaplacianOfGaussiansFilterTool;

#[derive(Clone, Copy)]
enum Phase3Op {
    FastAlmostGaussian,
    EdgePreservingMean,
    Unsharp,
    DiffOfGaussians,
    Adaptive,
    Lee,
    RefinedLee,
    EnhancedLee,
    ConservativeSmoothing,
    Olympic,
    KNearestMean,
    HighPassMedian,
    LaplacianOfGaussians,
}

impl Phase3Op {
    fn id(self) -> &'static str {
        match self {
            Self::FastAlmostGaussian => "fast_almost_gaussian_filter",
            Self::EdgePreservingMean => "edge_preserving_mean_filter",
            Self::Unsharp => "unsharp_masking",
            Self::DiffOfGaussians => "diff_of_gaussians_filter",
            Self::Adaptive => "adaptive_filter",
            Self::Lee => "lee_filter",
            Self::RefinedLee => "refined_lee_filter",
            Self::EnhancedLee => "enhanced_lee_filter",
            Self::ConservativeSmoothing => "conservative_smoothing_filter",
            Self::Olympic => "olympic_filter",
            Self::KNearestMean => "k_nearest_mean_filter",
            Self::HighPassMedian => "high_pass_median_filter",
            Self::LaplacianOfGaussians => "laplacian_of_gaussians_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::FastAlmostGaussian => "Fast Almost Gaussian Filter",
            Self::EdgePreservingMean => "Edge Preserving Mean Filter",
            Self::Unsharp => "Unsharp Masking",
            Self::DiffOfGaussians => "Difference of Gaussians Filter",
            Self::Adaptive => "Adaptive Filter",
            Self::Lee => "Lee Filter",
            Self::RefinedLee => "Refined Lee Filter",
            Self::EnhancedLee => "Enhanced Lee Filter",
            Self::ConservativeSmoothing => "Conservative Smoothing Filter",
            Self::Olympic => "Olympic Filter",
            Self::KNearestMean => "K-Nearest Mean Filter",
            Self::HighPassMedian => "High-Pass Median Filter",
            Self::LaplacianOfGaussians => "Laplacian of Gaussians Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::FastAlmostGaussian => "Performs a fast approximation to Gaussian smoothing.",
            Self::EdgePreservingMean => {
                "Performs thresholded edge-preserving mean filtering."
            }
            Self::Unsharp => "Performs edge-enhancing unsharp masking.",
            Self::DiffOfGaussians => {
                "Performs Difference-of-Gaussians band-pass filtering."
            }
            Self::Adaptive => {
                "Performs adaptive thresholded mean replacement based on local z-scores."
            }
            Self::Lee => {
                "Performs Lee sigma filtering using in-range neighborhood averaging."
            }
            Self::RefinedLee => {
                "Performs Refined Lee filtering with edge-preserving sub-window homogeneity classification."
            }
            Self::EnhancedLee => {
                "Performs Enhanced Lee filtering using sigma-ratio weighting and ENL-dependent blending."
            }
            Self::ConservativeSmoothing => {
                "Performs conservative smoothing by clipping impulse outliers to neighborhood bounds."
            }
            Self::Olympic => {
                "Performs Olympic smoothing by averaging local values excluding min and max."
            }
            Self::KNearestMean => {
                "Performs edge-preserving k-nearest neighbor mean smoothing."
            }
            Self::HighPassMedian => {
                "Performs high-pass filtering by subtracting local median from center values."
            }
            Self::LaplacianOfGaussians => {
                "Performs Laplacian-of-Gaussians edge enhancement."
            }
        }
    }
}

impl FastAlmostGaussianFilterTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn load_raster(path: &str) -> Result<Arc<Raster>, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_arc_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}': store entry is missing",
                    id
                ))
            });
        }

        Raster::read(path)
            .map(Arc::new)
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

    fn metadata_for(op: Phase3Op) -> ToolMetadata {
        let mut params = vec![ToolParamSpec {
            name: "input",
            description: "Input raster path or typed raster object.",
            required: true,
        }];

        match op {
            Phase3Op::FastAlmostGaussian => {
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Approximate Gaussian sigma (>=1.8 recommended, default 1.8).",
                    required: false,
                });
            }
            Phase3Op::EdgePreservingMean => {
                params.push(ToolParamSpec {
                    name: "filter_size",
                    description: "Square neighborhood size in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "threshold",
                    description: "Maximum absolute neighbor difference to include in local mean (default 15.0).",
                    required: false,
                });
            }
            Phase3Op::Unsharp => {
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Gaussian sigma used by the blur mask (0.5-20.0, default 0.75).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "amount",
                    description: "Sharpening amount multiplier applied to residual (default 100.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "threshold",
                    description: "Minimum absolute residual to sharpen (default 0.0).",
                    required: false,
                });
            }
            Phase3Op::DiffOfGaussians => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Smaller Gaussian sigma (0.25-20.0, default 2.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "sigma2",
                    description: "Larger Gaussian sigma (0.5-20.0, default 4.0).",
                    required: false,
                });
            }
            Phase3Op::Adaptive => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "threshold",
                    description: "Absolute z-score threshold for mean replacement (default 2.0).",
                    required: false,
                });
            }
            Phase3Op::Lee => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Intensity inclusion half-width around center value (default 10.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "m_value",
                    description: "Minimum in-range sample count before fallback averaging (default 5.0).",
                    required: false,
                });
            }
            Phase3Op::RefinedLee => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
            }
            Phase3Op::EnhancedLee => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "enl",
                    description: "Equivalent number of looks parameter for sigma-ratio weighting (default 4.0).",
                    required: false,
                });
            }
            Phase3Op::ConservativeSmoothing => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 3).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 3).",
                    required: false,
                });
            }
            Phase3Op::Olympic => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
            }
            Phase3Op::KNearestMean => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 3).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 3).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "k",
                    description: "Number of nearest neighbours to average (default 5).",
                    required: false,
                });
            }
            Phase3Op::HighPassMedian => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "sig_digits",
                    description: "Significant digits used for histogram binning (default 2).",
                    required: false,
                });
            }
            Phase3Op::LaplacianOfGaussians => {
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Gaussian sigma used by the LoG kernel (0.5-20.0, default 0.75).",
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

    fn manifest_for(op: Phase3Op) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));

        match op {
            Phase3Op::FastAlmostGaussian => {
                defaults.insert("sigma".to_string(), json!(1.8));
            }
            Phase3Op::EdgePreservingMean => {
                defaults.insert("filter_size".to_string(), json!(11));
                defaults.insert("threshold".to_string(), json!(15.0));
            }
            Phase3Op::Unsharp => {
                defaults.insert("sigma".to_string(), json!(0.75));
                defaults.insert("amount".to_string(), json!(100.0));
                defaults.insert("threshold".to_string(), json!(0.0));
            }
            Phase3Op::DiffOfGaussians => {
                defaults.insert("sigma1".to_string(), json!(2.0));
                defaults.insert("sigma2".to_string(), json!(4.0));
            }
            Phase3Op::Adaptive => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("threshold".to_string(), json!(2.0));
            }
            Phase3Op::Lee => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("sigma".to_string(), json!(10.0));
                defaults.insert("m_value".to_string(), json!(5.0));
            }
            Phase3Op::ConservativeSmoothing => {
                defaults.insert("filter_size_x".to_string(), json!(3));
                defaults.insert("filter_size_y".to_string(), json!(3));
            }
            Phase3Op::Olympic => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
            }
            Phase3Op::KNearestMean => {
                defaults.insert("filter_size_x".to_string(), json!(3));
                defaults.insert("filter_size_y".to_string(), json!(3));
                defaults.insert("k".to_string(), json!(5));
            }
            Phase3Op::HighPassMedian => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("sig_digits".to_string(), json!(2));
            }
            Phase3Op::LaplacianOfGaussians => {
                defaults.insert("sigma".to_string(), json!(0.75));
            }
            Phase3Op::RefinedLee => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
            }
            Phase3Op::EnhancedLee => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("enl".to_string(), json!(4.0));
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

    fn build_gaussian_kernel(sigma: f64) -> (Vec<isize>, Vec<isize>, Vec<f64>) {
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
        let mut weight_sum = 0.0f64;

        let mut a = 0usize;
        for row in 0..filter_size {
            for col in 0..filter_size {
                let x = col as isize - midpoint;
                let y = row as isize - midpoint;
                dx[a] = x;
                dy[a] = y;
                let w = recip_root_2_pi_times_sigma_d
                    * (-(x * x + y * y) as f64 / two_sigma_sqr_d).exp();
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

        (dx, dy, weights)
    }

    fn gaussian_blur_values(input: &Raster, sigma: f64, packed_rgb: bool) -> Vec<Vec<f64>> {
        let (dx, dy, weights) = Self::build_gaussian_kernel(sigma);
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        (0..bands)
            .into_par_iter()
            .map(|band_idx| {
                let band = band_idx as isize;
                let mut out = vec![nodata; rows * cols];
                out.par_chunks_mut(cols).enumerate().for_each(|(row_idx, out_row)| {
                    let row = row_idx as isize;
                    for col_idx in 0..cols {
                        let col = col_idx as isize;
                        let z0_raw = input.get(band, row, col);
                        if input.is_nodata(z0_raw) {
                            continue;
                        }
                        let mut sum = 0.0f64;
                        let mut zf = 0.0f64;
                        for i in 0..dx.len() {
                            let zn_raw = input.get(band, row + dy[i], col + dx[i]);
                            if input.is_nodata(zn_raw) {
                                continue;
                            }
                            let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                            sum += weights[i];
                            zf += weights[i] * zn;
                        }
                        if sum > 0.0 {
                            out_row[col_idx] = zf / sum;
                        }
                    }
                });
                out
            })
            .collect()
    }

    fn write_values_into_output(
        input: &Raster,
        output: &mut Raster,
        values: &[Vec<f64>],
        packed_rgb: bool,
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut rows_buf = vec![vec![nodata; cols]; rows];

            rows_buf
                .par_iter_mut()
                .enumerate()
                .for_each(|(r, out_row)| {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let v = values[band_idx][idx];
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
        }

        Ok(())
    }

    fn run_fast_almost_gaussian(
        input: &Raster,
        sigma: f64,
        packed_rgb: bool,
    ) -> Result<Raster, ToolError> {
        let n = 5isize;
        let sigma_eff = sigma.max(1.8);
        let w_ideal = (12.0 * sigma_eff * sigma_eff / n as f64 + 1.0).sqrt();
        let mut wl = w_ideal.floor() as isize;
        if wl % 2 == 0 {
            wl -= 1;
        }
        let wu = wl + 2;
        let m = ((12.0 * sigma_eff * sigma_eff
            - (n * wl * wl) as f64
            - (4 * n * wl) as f64
            - (3 * n) as f64)
            / (-4 * wl - 4) as f64)
            .round() as isize;

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let mut current: Vec<Vec<f64>> = (0..bands)
            .map(|band_idx| {
                let band = band_idx as isize;
                let mut v = vec![nodata; rows * cols];
                for r in 0..rows {
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if !input.is_nodata(z) {
                            v[r * cols + c] = if packed_rgb { value2i(z) } else { z };
                        }
                    }
                }
                v
            })
            .collect();

        for iter in 0..n {
            let width = if iter <= m { wl } else { wu } as usize;
            let radius = (width / 2) as isize;

            current = current
                .into_par_iter()
                .map(|band_vals| {
                    let stride = cols + 1;
                    let mut integral_sum = vec![0.0f64; (rows + 1) * (cols + 1)];
                    let mut integral_count = vec![0u32; (rows + 1) * (cols + 1)];

                    for r in 0..rows {
                        let mut row_sum = 0.0f64;
                        let mut row_count = 0u32;
                        let ir = (r + 1) * stride;
                        let ir_prev = r * stride;
                        for c in 0..cols {
                            let z = band_vals[r * cols + c];
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
                            let z0 = band_vals[r * cols + c];
                            if z0 == nodata {
                                continue;
                            }

                            let y1 = (r as isize - radius).max(0) as usize;
                            let y2 = (r as isize + radius).min((rows - 1) as isize) as usize;
                            let x1 = (c as isize - radius).max(0) as usize;
                            let x2 = (c as isize + radius).min((cols - 1) as isize) as usize;

                            let a = y1 * stride + x1;
                            let b = y1 * stride + (x2 + 1);
                            let cidx = (y2 + 1) * stride + x1;
                            let d = (y2 + 1) * stride + (x2 + 1);

                            let n = (integral_count[d] + integral_count[a] - integral_count[b]
                                - integral_count[cidx])
                                as f64;
                            if n > 0.0 {
                                let sum = integral_sum[d] + integral_sum[a]
                                    - integral_sum[b]
                                    - integral_sum[cidx];
                                out_row[c] = sum / n;
                            }
                        }
                    });

                    out
                })
                .collect();
        }

        let mut out = input.clone();
        Self::write_values_into_output(input, &mut out, &current, packed_rgb)?;
        Ok(out)
    }

    fn normalize_odd_size(v: usize, minimum: usize) -> usize {
        let mut s = v.max(minimum);
        if s % 2 == 0 {
            s += 1;
        }
        s
    }

    fn parse_window_sizes(args: &ToolArgs, default_x: usize, default_y: usize) -> (usize, usize, isize, isize) {
        let sx = Self::normalize_odd_size(
            args.get("filter_size_x").and_then(|v| v.as_u64()).unwrap_or(default_x as u64) as usize,
            3,
        );
        let sy = Self::normalize_odd_size(
            args.get("filter_size_y").and_then(|v| v.as_u64()).unwrap_or(default_y as u64) as usize,
            3,
        );
        let mx = (sx as f64 / 2.0).floor() as isize;
        let my = (sy as f64 / 2.0).floor() as isize;
        (sx, sy, mx, my)
    }

    fn min_max_by_band(input: &Raster, packed_rgb: bool) -> Vec<(f64, f64)> {
        (0..input.bands)
            .into_par_iter()
            .map(|band_idx| {
                let band = band_idx as isize;
                let mut min_v = f64::INFINITY;
                let mut max_v = f64::NEG_INFINITY;
                for r in 0..input.rows {
                    for c in 0..input.cols {
                        let z_raw = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z_raw) {
                            continue;
                        }
                        let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                        min_v = min_v.min(z);
                        max_v = max_v.max(z);
                    }
                }
                if min_v == f64::INFINITY {
                    (0.0, 0.0)
                } else {
                    (min_v, max_v)
                }
            })
            .collect()
    }

    fn run_with_op(op: Phase3Op, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info(&format!("running {}", op.id()));
        let input = Self::load_raster(&input_path)?;
        let rgb_mode = color_support::detect_rgb_mode(&input, false, true);
        let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && input.bands == 1;

        let output = match op {
            Phase3Op::FastAlmostGaussian => {
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.8)
                    .max(1.8);
                Self::run_fast_almost_gaussian(&input, sigma, packed_rgb)?
            }
            Phase3Op::EdgePreservingMean => {
                let mut filter_size = args
                    .get("filter_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(11) as usize;
                if filter_size < 3 {
                    filter_size = 3;
                }
                if filter_size % 2 == 0 {
                    filter_size += 1;
                }
                let threshold = args
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(15.0);

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let radius = (filter_size / 2) as isize;

                let mut out = input.as_ref().clone();
                let vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut band_buf = vec![nodata; rows * cols];
                        band_buf
                            .par_chunks_mut(cols)
                            .enumerate()
                            .for_each(|(r, row_buf)| {
                                for (c, cell) in row_buf.iter_mut().enumerate() {
                                    let z_raw = input.get(band, r as isize, c as isize);
                                    if input.is_nodata(z_raw) {
                                        continue;
                                    }
                                    *cell = if packed_rgb { value2i(z_raw) } else { z_raw };
                                }
                            });

                        let mut v = vec![nodata; rows * cols];
                        v.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                            let row = r as isize;
                            let row_offset = r * cols;
                            for c in 0..cols {
                                let z0 = band_buf[row_offset + c];
                                if z0 == nodata {
                                    continue;
                                }
                                let mut sum = 0.0;
                                let mut cnt = 0.0;
                                for ny in (row - radius)..=(row + radius) {
                                    if ny < 0 || ny >= rows as isize {
                                        continue;
                                    }
                                    let ny_offset = ny as usize * cols;
                                    for nx in (c as isize - radius)..=(c as isize + radius) {
                                        if nx < 0 || nx >= cols as isize {
                                            continue;
                                        }
                                        let zn = band_buf[ny_offset + nx as usize];
                                        if zn == nodata {
                                            continue;
                                        }
                                        if (zn - z0).abs() <= threshold {
                                            sum += zn;
                                            cnt += 1.0;
                                        }
                                    }
                                }
                                if cnt > 0.0 {
                                    row_out[c] = sum / cnt;
                                }
                            }
                        });
                        v
                    })
                    .collect();

                Self::write_values_into_output(&input, &mut out, &vals, packed_rgb)?;
                out
            }
            Phase3Op::Unsharp => {
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.75)
                    .clamp(0.5, 20.0);
                let amount = args
                    .get("amount")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(100.0);
                let threshold = args
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    .abs();

                let blur_vals = Self::gaussian_blur_values(&input, sigma, packed_rgb);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut v = vec![nodata; rows * cols];
                        v.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                            for c in 0..cols {
                                let idx = r * cols + c;
                                let z_raw = input.get(band, r as isize, c as isize);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let b = blur_vals[band_idx][idx];
                                if b == nodata {
                                    row_out[c] = z;
                                    continue;
                                }
                                let diff = z - b;
                                row_out[c] = if diff.abs() > threshold {
                                    z + diff * amount
                                } else {
                                    z
                                };
                            }
                        });
                        v
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::DiffOfGaussians => {
                let mut sigma1 = args
                    .get("sigma1")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(2.0)
                    .clamp(0.25, 20.0);
                let mut sigma2 = args
                    .get("sigma2")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(4.0)
                    .clamp(0.5, 20.0);
                if (sigma1 - sigma2).abs() < f64::EPSILON {
                    return Err(ToolError::Validation(
                        "sigma1 and sigma2 must not be equal".to_string(),
                    ));
                }
                if sigma1 > sigma2 {
                    std::mem::swap(&mut sigma1, &mut sigma2);
                }

                let g1 = Self::gaussian_blur_values(&input, sigma1, packed_rgb);
                let g2 = Self::gaussian_blur_values(&input, sigma2, packed_rgb);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let mut v = vec![nodata; rows * cols];
                        v.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                            for c in 0..cols {
                                let idx = r * cols + c;
                                let a = g1[band_idx][idx];
                                let b = g2[band_idx][idx];
                                if a != nodata && b != nodata {
                                    row_out[c] = a - b;
                                }
                            }
                        });
                        v
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::Adaptive => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let threshold = args
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(2.0)
                    .abs();

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let mins = Self::min_max_by_band(&input, packed_rgb)
                    .into_iter()
                    .map(|(min_v, _)| min_v)
                    .collect::<Vec<_>>();

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let min_val = mins[band_idx];
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let mut n = 0.0;
                                let mut sum = 0.0;
                                let mut sum2 = 0.0;
                                for ny in (row - my)..=(row + my) {
                                    for nx in (col - mx)..=(col + mx) {
                                        let zn_raw = input.get(band, ny, nx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        let zr = zn - min_val;
                                        sum += zr;
                                        sum2 += zr * zr;
                                        n += 1.0;
                                    }
                                }
                                if n <= 0.0 {
                                    continue;
                                }
                                let variance = (sum2 - (sum * sum) / n) / n;
                                if variance > 0.0 {
                                    let s = variance.sqrt();
                                    let mean = sum / n + min_val;
                                    out_row[c] = if ((z - mean) / s).abs() > threshold { mean } else { z };
                                } else {
                                    out_row[c] = z;
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::Lee => {
                let (sx, sy, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(10.0)
                    .abs();
                let mut m_value = args
                    .get("m_value")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(5.0);
                let max_cells = (sx * sy) as f64;
                if m_value > max_cells {
                    m_value = max_cells;
                }

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let n8 = [
                    (-1isize, -1isize),
                    (-1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                    (1, 0),
                    (1, -1),
                    (0, -1),
                ];

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let lo = z - sigma;
                                let hi = z + sigma;

                                let mut sum = 0.0;
                                let mut n = 0.0;
                                for ny in (row - my)..=(row + my) {
                                    for nx in (col - mx)..=(col + mx) {
                                        let zn_raw = input.get(band, ny, nx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        if zn >= lo && zn <= hi {
                                            sum += zn;
                                            n += 1.0;
                                        }
                                    }
                                }

                                if n > m_value {
                                    out_row[c] = sum / n;
                                } else {
                                    let mut s2 = 0.0;
                                    let mut n2 = 0.0;
                                    for (dy, dx) in &n8 {
                                        let zn_raw = input.get(band, row + *dy, col + *dx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        s2 += zn;
                                        n2 += 1.0;
                                    }
                                    if n2 > 0.0 {
                                        out_row[c] = s2 / n2;
                                    }
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::RefinedLee => {
                let (_sx, _sy, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };

                                // Scan 3x3 sub-windows within the filter kernel
                                let mut min_cov = f64::INFINITY;
                                let mut best_window_sum = 0.0;
                                let mut best_window_count = 0.0;

                                // 8 directional 3x3 windows centered at different positions
                                let window_offsets = [
                                    (-1, -1), (0, -1), (-1, 0), (0, 0),
                                    (-1, 1), (0, 1), (1, -1), (1, 0),
                                ];

                                for (wy, wx) in &window_offsets {
                                    let mut sum = 0.0;
                                    let mut sum2 = 0.0;
                                    let mut n = 0.0;
                                    for dy in -1..=1 {
                                        for dx in -1..=1 {
                                            let ny = row + *wy + dy;
                                            let nx = col + *wx + dx;
                                            let zn_raw = input.get(band, ny, nx);
                                            if input.is_nodata(zn_raw) {
                                                continue;
                                            }
                                            let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                            sum += zn;
                                            sum2 += zn * zn;
                                            n += 1.0;
                                        }
                                    }
                                    if n > 0.0 {
                                        let mean = sum / n;
                                        let variance = (sum2 - (sum * sum) / n) / n.max(1.0);
                                        let cov = if mean.abs() > f64::EPSILON {
                                            (variance.sqrt()) / mean.abs()
                                        } else {
                                            f64::INFINITY
                                        };
                                        if cov < min_cov {
                                            min_cov = cov;
                                            best_window_sum = sum;
                                            best_window_count = n;
                                        }
                                    }
                                }

                                if best_window_count > 0.0 {
                                    out_row[c] = best_window_sum / best_window_count;
                                } else {
                                    out_row[c] = z;
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::EnhancedLee => {
                let (_sx, _sy, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let enl = args
                    .get("enl")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(4.0)
                    .max(0.1);

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };

                                let mut sum = 0.0;
                                let mut sum2 = 0.0;
                                let mut n = 0.0;
                                for ny in (row - my)..=(row + my) {
                                    for nx in (col - mx)..=(col + mx) {
                                        let zn_raw = input.get(band, ny, nx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        sum += zn;
                                        sum2 += zn * zn;
                                        n += 1.0;
                                    }
                                }

                                if n > 1.0 {
                                    let mean = sum / n;
                                    let variance = (sum2 - (sum * sum) / n) / n;
                                    let sigma_ratio = variance / (mean.abs() + 1e-12);
                                    let weight = 1.0 / (1.0 + sigma_ratio * enl);
                                    out_row[c] = (1.0 - weight) * z + weight * mean;
                                } else {
                                    out_row[c] = z;
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::ConservativeSmoothing => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 3, 3);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let filter_width = 2 * mx as usize + 1;
                        let use_staged = filter_width > 3;
                        let band_buf = if use_staged {
                            let mut buf = vec![nodata; rows * cols];
                            buf.par_chunks_mut(cols)
                                .enumerate()
                                .for_each(|(r, row_buf)| {
                                    for (c, cell) in row_buf.iter_mut().enumerate() {
                                        let z_raw = input.get(band, r as isize, c as isize);
                                        if input.is_nodata(z_raw) {
                                            continue;
                                        }
                                        *cell = if packed_rgb { value2i(z_raw) } else { z_raw };
                                    }
                                });
                            Some(buf)
                        } else {
                            None
                        };

                        let fetch_value = |rr: usize, cc: usize, band_buf: &Option<Vec<f64>>| -> f64 {
                            if let Some(buf) = band_buf {
                                return buf[rr * cols + cc];
                            }
                            let z_raw = input.get(band, rr as isize, cc as isize);
                            if input.is_nodata(z_raw) {
                                nodata
                            } else if packed_rgb {
                                value2i(z_raw)
                            } else {
                                z_raw
                            }
                        };

                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            let start_row = (row - my).max(0) as usize;
                            let end_row = (row + my).min(rows as isize - 1) as usize;
                            let mut filter_min_vals = vec![f64::INFINITY; filter_width];
                            let mut filter_max_vals = vec![f64::NEG_INFINITY; filter_width];
                            let mut head = 0usize;

                            for c in 0..cols {
                                let z = fetch_value(r, c, &band_buf);
                                if z == nodata {
                                    continue;
                                }

                                if c > 0 {
                                    let mut col_min = f64::INFINITY;
                                    let mut col_max = f64::NEG_INFINITY;
                                    let new_col = c as isize + mx;
                                    if new_col >= 0 && new_col < cols as isize {
                                        let new_col = new_col as usize;
                                        for rr in start_row..=end_row {
                                            let zn = fetch_value(rr, new_col, &band_buf);
                                            if zn == nodata {
                                                continue;
                                            }
                                            if zn < col_min {
                                                col_min = zn;
                                            }
                                            if zn > col_max {
                                                col_max = zn;
                                            }
                                        }
                                    }
                                    filter_min_vals[head] = col_min;
                                    filter_max_vals[head] = col_max;
                                    head = (head + 1) % filter_width;
                                } else {
                                    for i in 0..filter_width {
                                        let cc = i as isize - mx;
                                        let mut col_min = f64::INFINITY;
                                        let mut col_max = f64::NEG_INFINITY;
                                        if cc >= 0 && cc < cols as isize {
                                            let cc = cc as usize;
                                            for rr in start_row..=end_row {
                                                let zn = fetch_value(rr, cc, &band_buf);
                                                if zn == nodata {
                                                    continue;
                                                }
                                                if zn < col_min {
                                                    col_min = zn;
                                                }
                                                if zn > col_max {
                                                    col_max = zn;
                                                }
                                            }
                                        }
                                        filter_min_vals[i] = col_min;
                                        filter_max_vals[i] = col_max;
                                    }
                                }

                                let mut min_v = f64::INFINITY;
                                let mut max_v = f64::NEG_INFINITY;
                                let mut min2_v = f64::INFINITY;
                                let mut max2_v = f64::NEG_INFINITY;
                                for i in 0..filter_width {
                                    let col_min = filter_min_vals[i];
                                    let col_max = filter_max_vals[i];
                                    if col_min < min_v {
                                        min2_v = min_v;
                                        min_v = col_min;
                                    } else if col_min < min2_v {
                                        min2_v = col_min;
                                    }
                                    if col_max > max_v {
                                        max2_v = max_v;
                                        max_v = col_max;
                                    } else if col_max > max2_v {
                                        max2_v = col_max;
                                    }
                                }

                                out_row[c] = if z > min_v && z < max_v {
                                    z
                                } else if z == min_v {
                                    if min2_v.is_finite() { min2_v } else { min_v }
                                } else if z == max_v {
                                    if max2_v.is_finite() { max2_v } else { max_v }
                                } else {
                                    z
                                };
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::Olympic => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let filter_width = 2 * mx as usize + 1;
                        let mut band_buf = vec![nodata; rows * cols];
                        band_buf
                            .par_chunks_mut(cols)
                            .enumerate()
                            .for_each(|(r, row_buf)| {
                                for (c, cell) in row_buf.iter_mut().enumerate() {
                                    let z_raw = input.get(band, r as isize, c as isize);
                                    if input.is_nodata(z_raw) {
                                        continue;
                                    }
                                    *cell = if packed_rgb { value2i(z_raw) } else { z_raw };
                                }
                            });

                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let start_row = (r as isize - my).max(0) as usize;
                            let end_row = (r as isize + my).min(rows as isize - 1) as usize;
                            let mut filter_min_vals = vec![f64::INFINITY; filter_width];
                            let mut filter_max_vals = vec![f64::NEG_INFINITY; filter_width];
                            let mut filter_totals = vec![0.0; filter_width];
                            let mut filter_counts = vec![0usize; filter_width];
                            let mut head = 0usize;

                            for c in 0..cols {
                                let z = band_buf[r * cols + c];
                                if z == nodata {
                                    continue;
                                }

                                if c > 0 {
                                    let mut col_min = f64::INFINITY;
                                    let mut col_max = f64::NEG_INFINITY;
                                    let mut col_total = 0.0;
                                    let mut col_count = 0usize;
                                    let new_col = c as isize + mx;
                                    if new_col >= 0 && new_col < cols as isize {
                                        let new_col = new_col as usize;
                                        for rr in start_row..=end_row {
                                            let zn = band_buf[rr * cols + new_col];
                                            if zn == nodata {
                                                continue;
                                            }
                                            if zn < col_min {
                                                col_min = zn;
                                            }
                                            if zn > col_max {
                                                col_max = zn;
                                            }
                                            col_total += zn;
                                            col_count += 1;
                                        }
                                    }
                                    filter_min_vals[head] = col_min;
                                    filter_max_vals[head] = col_max;
                                    filter_totals[head] = col_total;
                                    filter_counts[head] = col_count;
                                    head = (head + 1) % filter_width;
                                } else {
                                    for i in 0..filter_width {
                                        let cc = i as isize - mx;
                                        let mut col_min = f64::INFINITY;
                                        let mut col_max = f64::NEG_INFINITY;
                                        let mut col_total = 0.0;
                                        let mut col_count = 0usize;
                                        if cc >= 0 && cc < cols as isize {
                                            let cc = cc as usize;
                                            for rr in start_row..=end_row {
                                                let zn = band_buf[rr * cols + cc];
                                                if zn == nodata {
                                                    continue;
                                                }
                                                if zn < col_min {
                                                    col_min = zn;
                                                }
                                                if zn > col_max {
                                                    col_max = zn;
                                                }
                                                col_total += zn;
                                                col_count += 1;
                                            }
                                        }
                                        filter_min_vals[i] = col_min;
                                        filter_max_vals[i] = col_max;
                                        filter_totals[i] = col_total;
                                        filter_counts[i] = col_count;
                                    }
                                }

                                let mut min_v = f64::INFINITY;
                                let mut max_v = f64::NEG_INFINITY;
                                let mut sum = 0.0;
                                let mut n = 0usize;
                                for i in 0..filter_width {
                                    let col_min = filter_min_vals[i];
                                    let col_max = filter_max_vals[i];
                                    if col_min < min_v {
                                        min_v = col_min;
                                    }
                                    if col_max > max_v {
                                        max_v = col_max;
                                    }
                                    sum += filter_totals[i];
                                    n += filter_counts[i];
                                }

                                out_row[c] = if n > 2 {
                                    (sum - min_v - max_v) / (n - 2) as f64
                                } else {
                                    sum / n as f64
                                };
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::KNearestMean => {
                let (sx, sy, mx, my) = Self::parse_window_sizes(args, 3, 3);
                let mut k = args.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                k += 1;
                let max_cells = sx * sy;
                if k > max_cells {
                    k = max_cells;
                }

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            let mut best_vals = vec![0.0; k];
                            let mut best_dists = vec![0.0; k];
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let mut best_len = 0usize;

                                if packed_rgb {
                                    for ny in (row - my)..=(row + my) {
                                        for nx in (col - mx)..=(col + mx) {
                                            let zn_raw = input.get(band, ny, nx);
                                            if input.is_nodata(zn_raw) {
                                                continue;
                                            }
                                            let zn = value2i(zn_raw);
                                            let diff2 = (zn - z) * (zn - z);
                                            if best_len < k {
                                                let mut insert_at = best_len;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                                best_len += 1;
                                            } else if diff2 < best_dists[k - 1] {
                                                let mut insert_at = k - 1;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                            }
                                        }
                                    }
                                } else {
                                    for ny in (row - my)..=(row + my) {
                                        for nx in (col - mx)..=(col + mx) {
                                            let zn = input.get(band, ny, nx);
                                            if input.is_nodata(zn) {
                                                continue;
                                            }
                                            let diff2 = (zn - z) * (zn - z);
                                            if best_len < k {
                                                let mut insert_at = best_len;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                                best_len += 1;
                                            } else if diff2 < best_dists[k - 1] {
                                                let mut insert_at = k - 1;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                            }
                                        }
                                    }
                                }

                                if best_len == 0 {
                                    continue;
                                }

                                let sum = best_vals[..best_len].iter().sum::<f64>();
                                out_row[c] = sum / best_len as f64;
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::HighPassMedian => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let mut sig_digits = args
                    .get("sig_digits")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as i32;
                if packed_rgb && sig_digits < 4 {
                    sig_digits = 4;
                }
                let multiplier = 10f64.powi(sig_digits);

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let minmax = Self::min_max_by_band(&input, packed_rgb);

                let out_vals: Result<Vec<Vec<f64>>, ToolError> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| -> Result<Vec<f64>, ToolError> {
                        let band = band_idx as isize;
                        let (band_min, band_max) = if packed_rgb {
                            (0.0, 1.0)
                        } else {
                            minmax[band_idx]
                        };
                        if !band_min.is_finite() || !band_max.is_finite() {
                            return Ok(vec![nodata; rows * cols]);
                        }

                        let min_bin = (band_min * multiplier).floor() as i64;
                        let max_bin = (band_max * multiplier).floor() as i64;
                        let num_bins_i64 = (max_bin - min_bin + 1).max(1);
                        let num_bins = usize::try_from(num_bins_i64).map_err(|_| {
                            ToolError::Execution(
                                "high-pass-median histogram bin count exceeds platform limits".to_string(),
                            )
                        })?;

                        let bin_nodata = i64::MIN;
                        let mut binned = vec![bin_nodata; rows * cols];
                        binned
                            .par_chunks_mut(cols)
                            .enumerate()
                            .for_each(|(r, row_bins)| {
                                for (c, cell_bin) in row_bins.iter_mut().enumerate() {
                                    let z_raw = input.get(band, r as isize, c as isize);
                                    if input.is_nodata(z_raw) {
                                        continue;
                                    }
                                    let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                    *cell_bin = (z * multiplier).floor() as i64 - min_bin;
                                }
                            });

                        let rows_isize = rows as isize;
                        let cols_isize = cols as isize;
                        let get_bin = |rr: isize, cc: isize| -> i64 {
                            if rr < 0 || rr >= rows_isize || cc < 0 || cc >= cols_isize {
                                return bin_nodata;
                            }
                            binned[rr as usize * cols + cc as usize]
                        };

                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            let start_row = row - my;
                            let end_row = row + my;
                            let mut histo = vec![0i64; num_bins];
                            let mut old_median = bin_nodata;
                            let mut median = bin_nodata;
                            let mut n = 0i64;
                            let mut n_less = 0i64;

                            for c in 0..cols {
                                let col = c as isize;
                                let center_bin = get_bin(row, col);
                                if center_bin == bin_nodata {
                                    old_median = bin_nodata;
                                    continue;
                                }

                                if old_median != bin_nodata {
                                    let trailing_col = col - mx - 1;
                                    let leading_col = col + mx;

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, trailing_col);
                                        if bv != bin_nodata {
                                            histo[bv as usize] -= 1;
                                            n -= 1;
                                            if bv < old_median {
                                                n_less -= 1;
                                            }
                                        }
                                    }

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, leading_col);
                                        if bv != bin_nodata {
                                            histo[bv as usize] += 1;
                                            n += 1;
                                            if bv < old_median {
                                                n_less += 1;
                                            }
                                        }
                                    }

                                    let target = n / 2;
                                    if n_less < target {
                                        let mut v = old_median;
                                        while v < num_bins_i64 {
                                            let hv = histo[v as usize];
                                            if n_less + hv >= target {
                                                median = v;
                                                break;
                                            }
                                            n_less += hv;
                                            v += 1;
                                        }
                                    } else {
                                        let mut v = old_median - 1;
                                        while v >= 0 {
                                            let hv = histo[v as usize];
                                            if n_less - hv >= target {
                                                n_less -= hv;
                                                v -= 1;
                                            } else {
                                                median = v + 1;
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    histo.fill(0);
                                    n = 0;
                                    n_less = 0;
                                    let start_col = col - mx;
                                    let end_col = col + mx;

                                    for cc in start_col..=end_col {
                                        for rr in start_row..=end_row {
                                            let bv = get_bin(rr, cc);
                                            if bv != bin_nodata {
                                                histo[bv as usize] += 1;
                                                n += 1;
                                            }
                                        }
                                    }

                                    let target = n / 2;
                                    let mut acc = 0i64;
                                    for (i, hv) in histo.iter().enumerate() {
                                        acc += *hv;
                                        if acc >= target {
                                            median = i as i64;
                                            break;
                                        }
                                        n_less = acc;
                                    }
                                }

                                if n > 0 {
                                    out_row[c] = (center_bin - median) as f64 / multiplier;
                                }
                                old_median = median;
                            }
                        });

                        Ok(out)
                    })
                    .collect();
                let out_vals = out_vals?;

                if packed_rgb {
                    let rows = input.rows;
                    let cols = input.cols;
                    let mut out = input.as_ref().clone();
                    for band_idx in 0..input.bands {
                        let band = band_idx as isize;
                        let mut rows_buf = vec![vec![nodata; cols]; rows];
                        rows_buf
                            .par_iter_mut()
                            .enumerate()
                            .for_each(|(r, out_row)| {
                                for c in 0..cols {
                                    let idx = r * cols + c;
                                    let v = out_vals[band_idx][idx];
                                    if v == nodata {
                                        continue;
                                    }
                                    let z0 = input.get(band, r as isize, c as isize);
                                    let (h, s, _) = value2hsi(z0);
                                    out_row[c] = hsi2value(h, s, v + 0.5);
                                }
                            });
                        for (r, row) in rows_buf.iter().enumerate() {
                            out
                                .set_row_slice(band, r as isize, row)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing row {}: {}",
                                        r, e
                                    ))
                                })?;
                        }
                    }
                    out
                } else {
                    let mut out = input.as_ref().clone();
                    Self::write_values_into_output(&input, &mut out, &out_vals, false)?;
                    out
                }
            }
            Phase3Op::LaplacianOfGaussians => {
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.75)
                    .clamp(0.5, 20.0);

                let recip = 1.0 / ((2.0 * PI).sqrt() * sigma);
                let two_sigma_sqr = 2.0 * sigma * sigma;
                let mut filter_size = 0usize;
                for i in 0..250usize {
                    let weight = recip * (-((i * i) as f64) / two_sigma_sqr).exp();
                    if weight <= 0.001 {
                        filter_size = i * 2 + 1;
                        break;
                    }
                }
                filter_size = Self::normalize_odd_size(filter_size, 3);
                let radius = (filter_size as f64 / 2.0).floor() as isize;

                let term1 = -1.0 / (PI * sigma.powi(4));
                let mut kernel = Vec::with_capacity(filter_size * filter_size);
                for ry in 0..filter_size {
                    for rx in 0..filter_size {
                        let x = rx as isize - radius;
                        let y = ry as isize - radius;
                        let dist2 = (x * x + y * y) as f64;
                        let term2 = 1.0 - dist2 / two_sigma_sqr;
                        let term3 = (-dist2 / two_sigma_sqr).exp();
                        kernel.push((y, x, term1 * term2 * term3));
                    }
                }

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let mut weighted_sum = 0.0;
                                let mut weight_sum = 0.0;
                                if packed_rgb {
                                    for &(dy, dx, w) in &kernel {
                                        let zn_raw = input.get(band, row + dy, col + dx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = value2i(zn_raw);
                                        weighted_sum += w * zn;
                                        weight_sum += w;
                                    }
                                } else {
                                    for &(dy, dx, w) in &kernel {
                                        let zn = input.get(band, row + dy, col + dx);
                                        if input.is_nodata(zn) {
                                            continue;
                                        }
                                        weighted_sum += w * zn;
                                        weight_sum += w;
                                    }
                                }
                                if weight_sum != 0.0 {
                                    out_row[c] = weighted_sum / weight_sum;
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                if packed_rgb {
                    // Legacy behavior computes LoG on intensity and writes scalar results.
                    Self::write_values_into_output(&input, &mut out, &out_vals, false)?;
                } else {
                    Self::write_values_into_output(&input, &mut out, &out_vals, false)?;
                }
                out
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

macro_rules! define_phase3_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                FastAlmostGaussianFilterTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                FastAlmostGaussianFilterTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = FastAlmostGaussianFilterTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                FastAlmostGaussianFilterTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_phase3_tool!(FastAlmostGaussianFilterTool, Phase3Op::FastAlmostGaussian);
define_phase3_tool!(EdgePreservingMeanFilterTool, Phase3Op::EdgePreservingMean);
define_phase3_tool!(UnsharpMaskingTool, Phase3Op::Unsharp);
define_phase3_tool!(DiffOfGaussiansFilterTool, Phase3Op::DiffOfGaussians);
define_phase3_tool!(AdaptiveFilterTool, Phase3Op::Adaptive);
define_phase3_tool!(LeeFilterTool, Phase3Op::Lee);
define_phase3_tool!(RefinedLeeFilterTool, Phase3Op::RefinedLee);
define_phase3_tool!(EnhancedLeeFilterTool, Phase3Op::EnhancedLee);
define_phase3_tool!(ConservativeSmoothingFilterTool, Phase3Op::ConservativeSmoothing);
define_phase3_tool!(OlympicFilterTool, Phase3Op::Olympic);
define_phase3_tool!(KNearestMeanFilterTool, Phase3Op::KNearestMean);
define_phase3_tool!(HighPassMedianFilterTool, Phase3Op::HighPassMedian);
define_phase3_tool!(LaplacianOfGaussiansFilterTool, Phase3Op::LaplacianOfGaussians);

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
    fn fast_almost_gaussian_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(2.0));
        let out = run_with_memory(
            &FastAlmostGaussianFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn edge_preserving_mean_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size".to_string(), json!(7));
        args.insert("threshold".to_string(), json!(1.0));
        let out = run_with_memory(
            &EdgePreservingMeanFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn unsharp_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(1.0));
        args.insert("amount".to_string(), json!(100.0));
        args.insert("threshold".to_string(), json!(0.0));
        let out = run_with_memory(&UnsharpMaskingTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn dog_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("sigma1".to_string(), json!(2.0));
        args.insert("sigma2".to_string(), json!(4.0));
        let out = run_with_memory(
            &DiffOfGaussiansFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }

    #[test]
    fn adaptive_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("threshold".to_string(), json!(2.0));
        let out = run_with_memory(&AdaptiveFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn lee_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("sigma".to_string(), json!(10.0));
        args.insert("m_value".to_string(), json!(5.0));
        let out = run_with_memory(&LeeFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn conservative_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        let out = run_with_memory(
            &ConservativeSmoothingFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn olympic_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        let out = run_with_memory(&OlympicFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn knearest_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        args.insert("k".to_string(), json!(5));
        let out = run_with_memory(&KNearestMeanFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn high_pass_median_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("sig_digits".to_string(), json!(2));
        let out = run_with_memory(&HighPassMedianFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }

    #[test]
    fn log_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(0.75));
        let out = run_with_memory(
            &LaplacianOfGaussiansFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }
}
