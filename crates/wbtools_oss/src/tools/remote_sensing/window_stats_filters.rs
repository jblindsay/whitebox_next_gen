use std::collections::BTreeMap;
use std::sync::Arc;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, PercentCoalescer,
    ProgressSink, Tool, ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample,
    ToolManifest, ToolMetadata, ToolParamDescriptor, ToolParamSpec, ToolRunResult,
    ToolStability,
};
use wbraster::{Raster, RasterFormat};

use crate::memory_store;

pub struct MeanFilterTool;
pub struct TotalFilterTool;
pub struct StandardDeviationFilterTool;
pub struct MinimumFilterTool;
pub struct MaximumFilterTool;
pub struct RangeFilterTool;

const WINDOW_STATS_PAR_ROW_BATCH: usize = 64;

#[derive(Clone, Copy)]
enum WindowOp {
    Mean,
    Total,
    StdDev,
    Min,
    Max,
    Range,
}

impl WindowOp {
    fn id(self) -> &'static str {
        match self {
            Self::Mean => "mean_filter",
            Self::Total => "total_filter",
            Self::StdDev => "standard_deviation_filter",
            Self::Min => "minimum_filter",
            Self::Max => "maximum_filter",
            Self::Range => "range_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Mean => "Mean Filter",
            Self::Total => "Total Filter",
            Self::StdDev => "Standard Deviation Filter",
            Self::Min => "Minimum Filter",
            Self::Max => "Maximum Filter",
            Self::Range => "Range Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Mean => r#"Computes moving-window mean (average) for each pixel. Fundamental smoothing operation reducing local noise while blurring sharp transitions. Output represents local central tendency. Widely used for preprocessing, noise reduction, and multi-scale analysis.

Mean filtering is the most common low-pass smoothing operation. Highly sensitive to outliers (extreme values can distort results), making median filter preferable for noisy data. Computationally efficient. Filter size controls smoothing extent: small (3×3) preserves detail, large (31×31+) creates heavily smoothed surface. Often applied iteratively or at multiple scales for multi-resolution analysis.

Applications: (1) Basic noise reduction, (2) Preprocessing before feature detection (smooths false positives), (3) Multi-scale analysis (apply at 3×3, 11×11, 31×31), (4) Temporal smoothing (combining scenes), (5) Baseline for other statistical operations. Compare with median (non-linear, preserves edges) for improved edge preservation."#,
            Self::Total => r#"Computes moving-window sum (total) of pixel values in neighborhood. Integrates local signal strength. Applications depend on data semantics: for counts/densities, total reveals local density patterns; for precipitation, total reveals basin-scale accumulation; for reflectance, total is proportional to local target size.

Total filtering has different interpretations by domain. In count/population data, total reveals clustering and hotspots. In elevation data, total is rarely used (sum has no geomorphological meaning). In spectral analysis, total can reveal multi-band signal strength. Often used as intermediate step (e.g., divide by neighborhood cell count to compute mean, or compare with neighboring totals for local heterogeneity detection).

Applications: (1) Hotspot detection in count data (high total = clusters), (2) Basin/watershed accumulation models, (3) Integration of distributed measurements, (4) Intermediate calculation (total/N = mean), (5) Signal strength aggregation in multi-sensor mosaics."#,
            Self::StdDev => r#"Computes moving-window standard deviation, measuring local value variation/dispersion. High stdev = diverse values (rough/heterogeneous), low stdev = uniform values (smooth/homogeneous). Reveals texture, roughness, and variability patterns. Critical for uncertainty quantification and quality assessment.

Standard deviation is more robust than range for characterizing local variation (not biased by single outlier). Enables classification of areas by texture: steep slopes (high stdev), gentle slopes (low stdev); forests (high stdev), grasslands (low stdev). Often normalized (coefficient of variation = stdev/mean) to enable comparison across data with different value ranges. Can be computed from histogram (variance = mean_of_squares - square_of_mean).

Applications: (1) Texture mapping (roughness/heterogeneity analysis), (2) Uncertainty quantification in noisy data, (3) Quality assessment (uniform background = low stdev, feature-rich areas = high), (4) Classification confidence (high stdev = mixed/uncertain classes), (5) Multi-band heterogeneity (stack stdevs from each band). Typical workflow: compute stdev at multiple scales→compare pattern changes across scales→identify characteristic scales."#,
            Self::Min => r#"Computes moving-window minimum value, revealing local lows and troughs. Erosion operator in morphological image processing. Useful for detecting valley floors, depressions, and minimum-altitude features. Sensitive to single outlier (one low value in window produces low output).

Minimum filter is the morphological "erosion" operator—shrinks light regions and expands dark regions. When applied repeatedly (multi-pass erosion), creates smoothed valleys and isolated features disappear. Combined with maximum filter (dilation) enables opening (erosion then dilation) and closing (dilation then erosion) operations. Often used in multi-scale decomposition: compare min at 3×3, 11×11, 31×31 to identify feature scales.

Applications: (1) Morphological erosion for size-based filtering, (2) Opening via erosion→dilation to remove small noise objects, (3) Depression/valley identification in terrain, (4) Local floor level in bathymetry/DEM, (5) Multi-scale feature analysis (compare erosion across scales). Typical workflow: minimum→comparison with maximum→opening or closing depending on feature type."#,
            Self::Max => r#"Computes moving-window maximum value, revealing local peaks and ridges. Dilation operator in morphological image processing. Useful for detecting peaks, ridgelines, and maximum-amplitude features. Sensitive to single outlier (one high value in window produces high output).

Maximum filter is the morphological "dilation" operator—expands light regions and shrinks dark regions. When applied repeatedly (multi-pass dilation), creates smoothed peaks and isolated features grow to fill their neighborhoods. Combined with minimum filter enables closing (dilation then erosion) and opening (erosion then dilation). Essential for morphological feature detection and multi-scale analysis.

Applications: (1) Morphological dilation for size-based filtering, (2) Closing via dilation→erosion to fill small holes, (3) Peak/ridge identification in terrain and imagery, (4) Local ceiling level in bathymetry/DEM, (5) Multi-scale feature analysis (compare dilation across scales). Typical workflow: maximum→comparison with minimum→closing or opening depending on feature type."#,
            Self::Range => r#"Computes moving-window range (maximum - minimum), revealing local value spread independent of mean level. Simple heterogeneity metric: high range = diverse values, low range = uniform values. Simpler than standard deviation but equally informative for many applications, and more robust to distribution shape.

Range is computationally efficient (requires only two comparisons). Particularly useful for detecting transitions/boundaries where range spikes indicate contrast zones. Less sensitive to distribution shape than stdev (stdev emphasizes outliers, range only uses extremes). Normalized range (range/mean) enables cross-band comparison like coefficient of variation enables cross-scale comparison.

Applications: (1) Texture/contrast mapping (easy interpretation: high range = rough/contrasted), (2) Boundary detection via range peaks, (3) Computational efficiency alternative to stdev, (4) Quality control (uniform background low range, feature areas high range), (5) Roughness/variability in generic data. Typical workflow: compute range→threshold to identify transition zones→vectorize high-range boundaries."#,
        }
    }

    fn processing_message(self) -> &'static str {
        match self {
            Self::Mean => "applying moving-window mean",
            Self::Total => "applying moving-window total",
            Self::StdDev => "applying moving-window standard deviation",
            Self::Min => "applying moving-window minimum",
            Self::Max => "applying moving-window maximum",
            Self::Range => "applying moving-window range",
        }
    }

    fn tags(self) -> Vec<String> {
        vec![
            "remote_sensing".to_string(),
            "raster".to_string(),
            "filter".to_string(),
            "moving_window".to_string(),
            self.id().to_string(),
            "legacy-port".to_string(),
        ]
    }
}

impl MeanFilterTool {
    fn parse_window_sizes(args: &ToolArgs) -> (usize, usize) {
        let mut filter_x = args
            .get("filter_size_x")
            .and_then(|v| v.as_u64())
            .or_else(|| args.get("filterx").and_then(|v| v.as_u64()))
            .unwrap_or(11) as usize;
        let mut filter_y = args
            .get("filter_size_y")
            .and_then(|v| v.as_u64())
            .or_else(|| args.get("filtery").and_then(|v| v.as_u64()))
            .unwrap_or(filter_x as u64) as usize;

        if filter_x < 3 {
            filter_x = 3;
        }
        if filter_y < 3 {
            filter_y = 3;
        }
        if filter_x % 2 == 0 {
            filter_x += 1;
        }
        if filter_y % 2 == 0 {
            filter_y += 1;
        }
        (filter_x, filter_y)
    }

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

    fn metadata_for(op: WindowOp) -> ToolMetadata {
        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "filter_size_x",
                    description: "Window width in pixels (odd integer, default 11). Alias: filterx.",
                    required: false,
                },
                ToolParamSpec {
                    name: "filter_size_y",
                    description: "Window height in pixels (odd integer, default = filter_size_x). Alias: filtery.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest_for(op: WindowOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("filter_size_x".to_string(), json!(11));
        example_args.insert("filter_size_y".to_string(), json!(11));
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "filter_size_x".to_string(),
                    description: "Window width in pixels (odd integer, default 11). Alias: filterx.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "filter_size_y".to_string(),
                    description: "Window height in pixels (odd integer, default = filter_size_x). Alias: filtery.".to_string(),
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
                name: format!("basic_{}", op.id()),
                description: format!("Applies {} with an 11x11 neighborhood.", op.id()),
                args: example_args,
            }],
            tags: op.tags(),
            stability: ToolStability::Stable,
        }
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

    fn run_with_integral_op(
        input: &Raster,
        output: &mut Raster,
        filter_x: usize,
        filter_y: usize,
        op: WindowOp,
        progress: &dyn ProgressSink,
        compute_progress: &PercentCoalescer,
        done_rows: &mut usize,
        total_rows: usize,
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let half_x = (filter_x / 2) as isize;
        let half_y = (filter_y / 2) as isize;

        for band_idx in 0..bands {
            let band = band_idx as isize;

            let stride = cols + 1;
            let mut integral_sum = vec![0.0f64; (rows + 1) * (cols + 1)];
            let mut integral_sum_sq = vec![0.0f64; (rows + 1) * (cols + 1)];
            let mut integral_count = vec![0u32; (rows + 1) * (cols + 1)];

            for r in 0..rows {
                let mut row_sum = 0.0f64;
                let mut row_sum_sq = 0.0f64;
                let mut row_count = 0u32;
                let ir = (r + 1) * stride;
                let ir_prev = r * stride;
                for c in 0..cols {
                    let z = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z) {
                        row_sum += z;
                        row_sum_sq += z * z;
                        row_count += 1;
                    }
                    let idx = ir + (c + 1);
                    integral_sum[idx] = integral_sum[ir_prev + (c + 1)] + row_sum;
                    integral_sum_sq[idx] = integral_sum_sq[ir_prev + (c + 1)] + row_sum_sq;
                    integral_count[idx] = integral_count[ir_prev + (c + 1)] + row_count;
                }
            }

            let mut row_start = 0usize;
            while row_start < rows {
                let row_end = (row_start + WINDOW_STATS_PAR_ROW_BATCH).min(rows);
                let row_data: Vec<(usize, Vec<f64>)> = (row_start..row_end)
                    .into_par_iter()
                    .map(|r| {
                        let mut row_out = vec![nodata; cols];
                        for c in 0..cols {
                            let z_center = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z_center) {
                                continue;
                            }

                            let y1 = (r as isize - half_y).max(0) as usize;
                            let y2 = (r as isize + half_y).min((rows - 1) as isize) as usize;
                            let x1 = (c as isize - half_x).max(0) as usize;
                            let x2 = (c as isize + half_x).min((cols - 1) as isize) as usize;

                            let a = y1 * stride + x1;
                            let b = y1 * stride + (x2 + 1);
                            let cidx = (y2 + 1) * stride + x1;
                            let d = (y2 + 1) * stride + (x2 + 1);

                            let n = (integral_count[d] + integral_count[a] - integral_count[b] - integral_count[cidx]) as f64;
                            if n <= 0.0 {
                                row_out[c] = 0.0;
                                continue;
                            }

                            let sum = integral_sum[d] + integral_sum[a] - integral_sum[b] - integral_sum[cidx];

                            row_out[c] = match op {
                                WindowOp::Total => sum,
                                WindowOp::Mean => sum / n,
                                WindowOp::StdDev => {
                                    let sum_sq = integral_sum_sq[d] + integral_sum_sq[a] - integral_sum_sq[b] - integral_sum_sq[cidx];
                                    let variance = (sum_sq - (sum * sum) / n) / n;
                                    if variance > 0.0 { variance.sqrt() } else { 0.0 }
                                }
                                _ => nodata,
                            };
                        }
                        (r, row_out)
                    })
                    .collect();

                for (r, row) in row_data {
                    output
                        .set_row_slice(band, r as isize, &row)
                        .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                    *done_rows += 1;
                    compute_progress.emit_unit_fraction(progress, *done_rows as f64 / total_rows as f64);
                }

                row_start = row_end;
            }
        }

        Ok(())
    }

    fn run_with_extrema_op(
        input: &Raster,
        output: &mut Raster,
        filter_x: usize,
        filter_y: usize,
        op: WindowOp,
        progress: &dyn ProgressSink,
        compute_progress: &PercentCoalescer,
        done_rows: &mut usize,
        total_rows: usize,
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let half_x = (filter_x / 2) as isize;
        let half_y = (filter_y / 2) as isize;
        let nodata_is_nan = nodata.is_nan();

        for band_idx in 0..bands {
            let band = band_idx as isize;

            // Pre-load the entire band into a flat row-major buffer in parallel.
            // Parallelising the fill means we do rows×cols input.get() calls spread
            // across all rayon threads, vs. filter_y×cols×rows calls in the hot path —
            // ~filter_y× fewer dispatched calls, fully parallelised.
            let mut band_buf = vec![nodata; rows * cols];
            band_buf
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_slice)| {
                    for c in 0..cols {
                        row_slice[c] = input.get(band, r as isize, c as isize);
                    }
                });

            // Inline nodata test used in the hot path (avoids is_nodata() call overhead).
            let is_nd = |z: f64| -> bool {
                if nodata_is_nan { z.is_nan() } else { z == nodata }
            };

            // Clamp a column index to [0, cols) for border handling.
            // An out-of-bounds column contributes no valid data (INFINITY / NEG_INFINITY).
            let cols_isize = cols as isize;
            let rows_isize = rows as isize;

            let mut out_buf = vec![nodata; rows * cols];
            out_buf
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(r, row_out)| {
                    let r_start = ((r as isize) - half_y).max(0) as usize;
                    let r_end   = ((r as isize) + half_y).min(rows_isize - 1) as usize;

                    // Compute min (and optionally max) for a single column cx over
                    // the clamped vertical window [r_start..=r_end].
                    // Returns INFINITY / NEG_INFINITY for OOB columns (no valid data).
                    let col_min = |cx: isize| -> f64 {
                        if cx < 0 || cx >= cols_isize { return f64::INFINITY; }
                        let cx = cx as usize;
                        let mut mn = f64::INFINITY;
                        let mut idx = r_start * cols + cx;
                        for _ in r_start..=r_end {
                            // idx is guaranteed in-bounds by clamped r_start/r_end and cx checks.
                            let z = unsafe { *band_buf.get_unchecked(idx) };
                            if !is_nd(z) && z < mn { mn = z; }
                            idx += cols;
                        }
                        mn
                    };
                    let col_max = |cx: isize| -> f64 {
                        if cx < 0 || cx >= cols_isize { return f64::NEG_INFINITY; }
                        let cx = cx as usize;
                        let mut mx = f64::NEG_INFINITY;
                        let mut idx = r_start * cols + cx;
                        for _ in r_start..=r_end {
                            // idx is guaranteed in-bounds by clamped r_start/r_end and cx checks.
                            let z = unsafe { *band_buf.get_unchecked(idx) };
                            if !is_nd(z) && z > mx { mx = z; }
                            idx += cols;
                        }
                        mx
                    };
                    let col_range = |cx: isize| -> (f64, f64) {
                        if cx < 0 || cx >= cols_isize { return (f64::INFINITY, f64::NEG_INFINITY); }
                        let cx = cx as usize;
                        let mut mn = f64::INFINITY;
                        let mut mx = f64::NEG_INFINITY;
                        let mut idx = r_start * cols + cx;
                        for _ in r_start..=r_end {
                            // idx is guaranteed in-bounds by clamped r_start/r_end and cx checks.
                            let z = unsafe { *band_buf.get_unchecked(idx) };
                            if !is_nd(z) {
                                if z < mn { mn = z; }
                                if z > mx { mx = z; }
                            }
                            idx += cols;
                        }
                        (mn, mx)
                    };

                    match op {
                        WindowOp::Min => {
                            let mut filter_mins = vec![f64::INFINITY; filter_x];
                            // Initialise the sliding cache for c=0: window is [-half_x..half_x].
                            for i in 0..filter_x {
                                filter_mins[i] = col_min(i as isize - half_x);
                            }
                            let mut head = 0usize;

                            for c in 0..cols {
                                if c > 0 {
                                    // Slide: evict oldest column (at head), add incoming right edge.
                                    filter_mins[head] = col_min(c as isize + half_x);
                                    head = (head + 1) % filter_x;
                                }
                                if is_nd(band_buf[r * cols + c]) { continue; }

                                let mut min_val = f64::INFINITY;
                                for v in &filter_mins { if *v < min_val { min_val = *v; } }
                                if min_val < f64::INFINITY { row_out[c] = min_val; }
                            }
                        }
                        WindowOp::Max => {
                            let mut filter_maxs = vec![f64::NEG_INFINITY; filter_x];
                            for i in 0..filter_x {
                                filter_maxs[i] = col_max(i as isize - half_x);
                            }
                            let mut head = 0usize;

                            for c in 0..cols {
                                if c > 0 {
                                    filter_maxs[head] = col_max(c as isize + half_x);
                                    head = (head + 1) % filter_x;
                                }
                                if is_nd(band_buf[r * cols + c]) { continue; }

                                let mut max_val = f64::NEG_INFINITY;
                                for v in &filter_maxs { if *v > max_val { max_val = *v; } }
                                if max_val > f64::NEG_INFINITY { row_out[c] = max_val; }
                            }
                        }
                        WindowOp::Range => {
                            let mut filter_mins = vec![f64::INFINITY; filter_x];
                            let mut filter_maxs = vec![f64::NEG_INFINITY; filter_x];
                            for i in 0..filter_x {
                                let (mn, mx) = col_range(i as isize - half_x);
                                filter_mins[i] = mn;
                                filter_maxs[i] = mx;
                            }
                            let mut head = 0usize;

                            for c in 0..cols {
                                if c > 0 {
                                    let (mn, mx) = col_range(c as isize + half_x);
                                    filter_mins[head] = mn;
                                    filter_maxs[head] = mx;
                                    head = (head + 1) % filter_x;
                                }
                                if is_nd(band_buf[r * cols + c]) { continue; }

                                let mut min_val = f64::INFINITY;
                                let mut max_val = f64::NEG_INFINITY;
                                for i in 0..filter_x {
                                    if filter_mins[i] < min_val { min_val = filter_mins[i]; }
                                    if filter_maxs[i] > max_val { max_val = filter_maxs[i]; }
                                }
                                if min_val < f64::INFINITY && max_val > f64::NEG_INFINITY {
                                    row_out[c] = max_val - min_val;
                                }
                            }
                        }
                        _ => unreachable!("run_with_extrema_op only supports min/max/range"),
                    }
                });

            for r in 0..rows {
                output
                    .set_row_slice(band, r as isize, &out_buf[r * cols..(r + 1) * cols])
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                *done_rows += 1;
                compute_progress.emit_unit_fraction(progress, *done_rows as f64 / total_rows as f64);
            }
        }

        Ok(())
    }

    fn run_with_op(op: WindowOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let (filter_x, filter_y) = Self::parse_window_sizes(args);

        ctx.progress.info(&format!("running {}", op.id()));
        ctx.progress.info("reading input raster");
        let input = Self::load_raster(&input_path)?;
        let mut output = input.as_ref().clone();
        let total_rows = (input.rows * input.bands).max(1);
        let mut done_rows = 0usize;
        let compute_progress = PercentCoalescer::new(1, 90);

        ctx.progress.info(op.processing_message());

        match op {
            WindowOp::Mean | WindowOp::Total | WindowOp::StdDev => {
                Self::run_with_integral_op(
                    &input,
                    &mut output,
                    filter_x,
                    filter_y,
                    op,
                    ctx.progress,
                    &compute_progress,
                    &mut done_rows,
                    total_rows,
                )?;
            }
            WindowOp::Min | WindowOp::Max | WindowOp::Range => {
                Self::run_with_extrema_op(
                    &input,
                    &mut output,
                    filter_x,
                    filter_y,
                    op,
                    ctx.progress,
                    &compute_progress,
                    &mut done_rows,
                    total_rows,
                )?;
            }
        }

        compute_progress.finish(ctx.progress);

        let output_locator = Self::write_or_store_output(output, output_path)?;

        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

macro_rules! define_window_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                MeanFilterTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                MeanFilterTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = MeanFilterTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                let _ = MeanFilterTool::parse_window_sizes(args);
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                MeanFilterTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_window_tool!(MeanFilterTool, WindowOp::Mean);
define_window_tool!(TotalFilterTool, WindowOp::Total);
define_window_tool!(StandardDeviationFilterTool, WindowOp::StdDev);
define_window_tool!(MinimumFilterTool, WindowOp::Min);
define_window_tool!(MaximumFilterTool, WindowOp::Max);
define_window_tool!(RangeFilterTool, WindowOp::Range);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use wbcore::{AllowAllCapabilities, ProgressSink, ToolContext};
    use wbraster::RasterConfig;

    struct NoopProgress;
    impl ProgressSink for NoopProgress {}

    struct RecordingProgress {
        percents: Mutex<Vec<f64>>,
    }

    impl RecordingProgress {
        fn new() -> Self {
            Self {
                percents: Mutex::new(Vec::new()),
            }
        }

        fn percents(&self) -> Vec<f64> {
            self.percents.lock().unwrap().clone()
        }
    }

    impl ProgressSink for RecordingProgress {
        fn progress(&self, pct: f64) {
            self.percents.lock().unwrap().push(pct);
        }
    }

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
    fn mean_filter_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(5));
        args.insert("filter_size_y".to_string(), json!(5));
        let out = run_with_memory(&MeanFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        for row in 0..25isize {
            for col in 0..25isize {
                assert!((out.get(0, row, col) - 10.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn min_max_and_range_constant_raster_expected_values() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(7));
        args.insert("filter_size_y".to_string(), json!(7));

        let min_out = run_with_memory(&MinimumFilterTool, &mut args.clone(), make_constant_raster(21, 21, 3.0));
        let max_out = run_with_memory(&MaximumFilterTool, &mut args.clone(), make_constant_raster(21, 21, 3.0));
        let rng_out = run_with_memory(&RangeFilterTool, &mut args, make_constant_raster(21, 21, 3.0));

        for row in 0..21isize {
            for col in 0..21isize {
                assert!((min_out.get(0, row, col) - 3.0).abs() < 1e-9);
                assert!((max_out.get(0, row, col) - 3.0).abs() < 1e-9);
                assert!(rng_out.get(0, row, col).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn stddev_filter_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(9));
        args.insert("filter_size_y".to_string(), json!(9));
        let out = run_with_memory(&StandardDeviationFilterTool, &mut args, make_constant_raster(30, 30, 42.0));
        for row in 0..30isize {
            for col in 0..30isize {
                assert!(out.get(0, row, col).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn total_filter_matches_neighborhood_sum_interior() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        let out = run_with_memory(&TotalFilterTool, &mut args, make_constant_raster(10, 10, 2.0));
        assert!((out.get(0, 5, 5) - 18.0).abs() < 1e-9);
        assert!((out.get(0, 0, 0) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn mean_filter_progress_is_monotonic_bounded_and_completes() {
        let input = make_constant_raster(1024, 1024, 10.0);
        let input_id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&input_id)),
        );
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));

        let caps = AllowAllCapabilities;
        let progress = RecordingProgress::new();
        let ctx = ToolContext {
            progress: &progress,
            capabilities: &caps,
        };

        let tool = MeanFilterTool;
        let _ = tool.run(&args, &ctx).expect("mean filter should run");

        let percents = progress.percents();
        assert!(!percents.is_empty(), "expected progress events");
        assert!(percents.len() <= 101, "progress events should be bounded to percent buckets");

        for w in percents.windows(2) {
            assert!(w[1] >= w[0], "progress should be monotonic non-decreasing");
        }

        let final_pct = *percents.last().unwrap();
        assert!((final_pct - 1.0).abs() < 1e-9, "final progress should be 100%");
    }
}
