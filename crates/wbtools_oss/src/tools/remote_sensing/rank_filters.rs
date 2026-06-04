use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, PercentCoalescer, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};

use crate::memory_store;

pub struct MedianFilterTool;
pub struct PercentileFilterTool;
pub struct MajorityFilterTool;
pub struct DiversityFilterTool;

const RANK_FILTER_PAR_ROW_BATCH: usize = 64;

#[derive(Clone, Copy)]
enum RankOp {
    Median,
    Percentile,
    Majority,
    Diversity,
}

impl RankOp {
    fn id(self) -> &'static str {
        match self {
            Self::Median => "median_filter",
            Self::Percentile => "percentile_filter",
            Self::Majority => "majority_filter",
            Self::Diversity => "diversity_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Median => "Median Filter",
            Self::Percentile => "Percentile Filter",
            Self::Majority => "Majority Filter",
            Self::Diversity => "Diversity Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Median => r#"Computes moving-window median value for each pixel, replacing with 50th percentile of neighborhood. Robust noise filter preserving edges (non-linear). Particularly effective for impulse noise (salt-and-pepper) removal while maintaining sharp boundaries. Output values are actual pixel values from neighborhood (not interpolated).

Median filtering is non-linear—critical advantage over mean filtering for noise reduction because it doesn't create new values or blur edges. Large filter sizes heavily smooth while preserving sharp transitions. Widely used in remote sensing, medical imaging, and SAR image processing. Computational cost increases with filter size but generally faster than bilateral or guided filters.

Applications: (1) SAR image speckle reduction (especially effective for phase coherence), (2) Salt-and-pepper noise removal, (3) Preprocessing before edge detection (reduces false edges), (4) Boundary preservation in classification preprocessing, (5) Radiometric correction for outlier values. Typical workflow: apply median→edge-enhanced output→threshold for feature extraction."#,
            Self::Percentile => r#"Computes local percentile rank of center cell elevation/value within moving window (0-100%). Analogous to Elevation Percentile for generic raster data. Measures relative position: output=0 indicates local minimum, output=100 indicates local maximum, output=50 indicates median. Reveals local position-in-distribution independently of absolute values.

Percentile filtering enables position-relative analysis. Useful for layering analysis: cells ranking high percentile (>80) in all bands indicate "bright" features; low percentile (<20) indicate "dark" features. Particularly useful for classification preprocessing—separates terrain/texture position rather than just magnitude. Often combined with statistical filters for multi-metric characterization.

Applications: (1) Relative brightness/darkness classification, (2) Texture characterization (high percentile variance = rough, low variance = smooth), (3) Local contrast enhancement (percentile-based normalization), (4) Landform identification similar to elevation percentile, (5) Multi-band texture analysis (apply percentile to each band, compare patterns)."#,
            Self::Majority => r#"Computes moving-window mode (most frequent value/class) for each pixel. Non-linear filter preserving categorical boundaries and dominant patterns. Particularly useful for classified imagery where output must remain within original class set (unlike mean filter which creates interpolated values). Essential for morphological cleaning of classification outputs.

Majority filtering is the mode-based equivalent of median filtering. For categorical data (classified imagery, land cover), majority preserves class definitions while smoothing noise. For continuous data, majority can reveal local peaks in value distribution. Often followed by minority class elimination (post-classification cleanup) to remove "salt-and-pepper" classification artifacts.

Applications: (1) Post-classification smoothing (removes small spurious class patches), (2) Majority class map from multi-classified outputs, (3) Vector data cleaning (class disaggregation), (4) Noise suppression in thresholded imagery, (5) Attribute smoothing in segmentation outputs. Typical workflow: classify→majority filter→minority elimination→final cleaned map."#,
            Self::Diversity => r#"Computes moving-window diversity as count of unique values/classes within neighborhood. Measures local heterogeneity: high diversity = varied terrain/classes, low diversity = homogeneous. Reveals texture, fragmentation, and pattern diversity. Often applied to classified or categorical imagery to identify transition/edge zones.

Diversity filter creates a metric of local variation independent of specific values. Useful for landscape ecology (habitat diversity, fragmentation metrics), classification quality assessment (high diversity = mixed/uncertain areas), and texture analysis. Applied to elevation data, diversity indicates roughness at neighborhood scale. Applied to classification, it identifies mixed/ecotone/boundary zones.

Applications: (1) Landscape fragmentation mapping (high diversity = diverse mosaic, low diversity = uniform patches), (2) Classification confidence/uncertainty assessment (high diversity = uncertain area), (3) Texture analysis (heterogeneity mapping), (4) Edge/boundary detection via diversity peaks, (5) Habitat diversity for ecological analysis (suitability depends on local diversity)."#,
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

    fn needs_sig_digits(self) -> bool {
        matches!(self, Self::Median | Self::Percentile)
    }
}

impl MedianFilterTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

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

    fn parse_sig_digits(args: &ToolArgs) -> i32 {
        args.get("sig_digits")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(2)
            .clamp(0, 9)
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

    fn metadata_for(op: RankOp) -> ToolMetadata {
        let mut params = vec![
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
        ];
        if op.needs_sig_digits() {
            params.push(ToolParamSpec {
                name: "sig_digits",
                description: "Number of significant digits used for quantized rank filtering (default 2).",
                required: false,
            });
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

    fn manifest_for(op: RankOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));
        if op.needs_sig_digits() {
            defaults.insert("sig_digits".to_string(), json!(2));
        }

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("filter_size_x".to_string(), json!(11));
        example_args.insert("filter_size_y".to_string(), json!(11));
        if op.needs_sig_digits() {
            example_args.insert("sig_digits".to_string(), json!(2));
        }
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));

        let mut params = vec![
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
        ];
        if op.needs_sig_digits() {
            params.push(ToolParamDescriptor {
                name: "sig_digits".to_string(),
                description: "Number of significant digits used for quantized rank filtering (default 2).".to_string(),
                required: false,
            });
        }
        params.push(ToolParamDescriptor {
            name: "output".to_string(),
            description: "Optional output path. If omitted, result is stored in memory.".to_string(),
            required: false,
        });

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
                description: format!("Applies {} with an 11x11 neighborhood.", op.id()),
                args: example_args,
            }],
            tags: op.tags(),
            stability: ToolStability::Experimental,
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

    fn run_with_op(op: RankOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let (filter_x, filter_y) = Self::parse_window_sizes(args);
        let sig_digits = Self::parse_sig_digits(args);

        let multiplier_rank = 10.0f64.powi(sig_digits);
        ctx.progress.info(&format!("running {}", op.id()));
        ctx.progress.info("reading input raster");

        let input = Self::load_raster(&input_path)?;
        let mut output = input.as_ref().clone();

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let half_x = (filter_x / 2) as isize;
        let half_y = (filter_y / 2) as isize;

        let total_rows = (rows * bands).max(1);
        let mut done_rows = 0usize;
        let compute_progress = PercentCoalescer::new(1, 90);

        for band_idx in 0..bands {
            let band = band_idx as isize;

            if matches!(op, RankOp::Median) {
                // Huang-style running median: build a quantized histogram for each row and
                // update it incrementally while sliding the window across columns.
                let mut band_min = f64::INFINITY;
                let mut band_max = f64::NEG_INFINITY;
                for r in 0..rows {
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        if z < band_min {
                            band_min = z;
                        }
                        if z > band_max {
                            band_max = z;
                        }
                    }
                }

                if !band_min.is_finite() || !band_max.is_finite() {
                    done_rows += rows;
                    compute_progress.emit_unit_fraction(
                        ctx.progress,
                        done_rows as f64 / total_rows as f64,
                    );
                    continue;
                }

                let min_bin = (band_min * multiplier_rank).floor() as i64;
                let max_bin = (band_max * multiplier_rank).floor() as i64;
                let num_bins_i64 = (max_bin - min_bin + 1).max(1);
                let num_bins = usize::try_from(num_bins_i64).map_err(|_| {
                    ToolError::Execution(
                        "median filter histogram bin count exceeds platform limits".to_string(),
                    )
                })?;

                let bin_nodata = i64::MIN;
                let mut binned = vec![bin_nodata; rows * cols];
                binned
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(r, row_bins)| {
                        for (c, cell_bin) in row_bins.iter_mut().enumerate() {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            *cell_bin = (z * multiplier_rank).floor() as i64 - min_bin;
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

                let mut row_start = 0usize;
                while row_start < rows {
                    let row_end = (row_start + RANK_FILTER_PAR_ROW_BATCH).min(rows);
                    let row_data: Vec<(usize, Vec<f64>)> = (row_start..row_end)
                        .into_par_iter()
                        .map(|r| {
                            let row = r as isize;
                            let mut row_out = vec![nodata; cols];
                            let mut histo = vec![0i64; num_bins];
                            let mut old_median = bin_nodata;
                            let mut median = bin_nodata;
                            let mut n = 0i64;
                            let mut n_less = 0i64;
                            let start_row = row - half_y;
                            let end_row = row + half_y;

                            for c in 0..cols {
                                let col = c as isize;
                                let center_bin = get_bin(row, col);
                                if center_bin == bin_nodata {
                                    old_median = bin_nodata;
                                    continue;
                                }

                                if old_median != bin_nodata {
                                    // Remove trailing column and add leading column as the
                                    // window slides one cell to the right.
                                    let trailing_col = col - half_x - 1;
                                    let leading_col = col + half_x;

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
                                    // Initialize histogram for the first valid cell in a row,
                                    // or after nodata resets.
                                    histo.fill(0);
                                    n = 0;
                                    n_less = 0;
                                    let start_col = col - half_x;
                                    let end_col = col + half_x;

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
                                    row_out[c] = (median + min_bin) as f64 / multiplier_rank;
                                }
                                old_median = median;
                            }

                            (r, row_out)
                        })
                        .collect();

                    for (r, row) in row_data {
                        output
                            .set_row_slice(band, r as isize, &row)
                            .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                        done_rows += 1;
                        compute_progress.emit_unit_fraction(
                            ctx.progress,
                            done_rows as f64 / total_rows as f64,
                        );
                    }

                    row_start = row_end;
                }

                continue;
            }

            if matches!(op, RankOp::Percentile) {
                // Huang-style running percentile rank: maintain a quantized histogram while
                // sliding the window and update the less-than-center count incrementally.
                let mut band_min = f64::INFINITY;
                let mut band_max = f64::NEG_INFINITY;
                for r in 0..rows {
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        if z < band_min {
                            band_min = z;
                        }
                        if z > band_max {
                            band_max = z;
                        }
                    }
                }

                if !band_min.is_finite() || !band_max.is_finite() {
                    done_rows += rows;
                    compute_progress.emit_unit_fraction(
                        ctx.progress,
                        done_rows as f64 / total_rows as f64,
                    );
                    continue;
                }

                let min_bin = (band_min * multiplier_rank).floor() as i64;
                let max_bin = (band_max * multiplier_rank).floor() as i64;
                let num_bins_i64 = (max_bin - min_bin + 1).max(1);
                let num_bins = usize::try_from(num_bins_i64).map_err(|_| {
                    ToolError::Execution(
                        "percentile filter histogram bin count exceeds platform limits".to_string(),
                    )
                })?;

                let bin_nodata = i64::MIN;
                let mut binned = vec![bin_nodata; rows * cols];
                binned
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(r, row_bins)| {
                        for (c, cell_bin) in row_bins.iter_mut().enumerate() {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            *cell_bin = (z * multiplier_rank).floor() as i64 - min_bin;
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

                let mut row_start = 0usize;
                while row_start < rows {
                    let row_end = (row_start + RANK_FILTER_PAR_ROW_BATCH).min(rows);
                    let row_data: Vec<(usize, Vec<f64>)> = (row_start..row_end)
                        .into_par_iter()
                        .map(|r| {
                            let row = r as isize;
                            let mut row_out = vec![nodata; cols];
                            let mut histo = vec![0i64; num_bins];
                            let mut old_center = bin_nodata;
                            let mut n = 0i64;
                            let mut n_less = 0i64;
                            let start_row = row - half_y;
                            let end_row = row + half_y;

                            for c in 0..cols {
                                let col = c as isize;
                                let center_bin = get_bin(row, col);
                                if center_bin == bin_nodata {
                                    old_center = bin_nodata;
                                    continue;
                                }

                                if old_center != bin_nodata {
                                    let trailing_col = col - half_x - 1;
                                    let leading_col = col + half_x;

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, trailing_col);
                                        if bv != bin_nodata {
                                            histo[bv as usize] -= 1;
                                            n -= 1;
                                            if bv < old_center {
                                                n_less -= 1;
                                            }
                                        }
                                    }

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, leading_col);
                                        if bv != bin_nodata {
                                            histo[bv as usize] += 1;
                                            n += 1;
                                            if bv < old_center {
                                                n_less += 1;
                                            }
                                        }
                                    }

                                    if old_center < center_bin {
                                        let mut m = 0i64;
                                        for v in old_center..center_bin {
                                            m += histo[v as usize];
                                        }
                                        n_less += m;
                                    } else if old_center > center_bin {
                                        let mut m = 0i64;
                                        for v in center_bin..old_center {
                                            m += histo[v as usize];
                                        }
                                        n_less -= m;
                                    }
                                } else {
                                    histo.fill(0);
                                    n = 0;
                                    n_less = 0;
                                    let start_col = col - half_x;
                                    let end_col = col + half_x;

                                    for cc in start_col..=end_col {
                                        for rr in start_row..=end_row {
                                            let bv = get_bin(rr, cc);
                                            if bv != bin_nodata {
                                                histo[bv as usize] += 1;
                                                n += 1;
                                                if bv < center_bin {
                                                    n_less += 1;
                                                }
                                            }
                                        }
                                    }
                                }

                                if n > 0 {
                                    row_out[c] = n_less as f64 / n as f64 * 100.0;
                                }
                                old_center = center_bin;
                            }

                            (r, row_out)
                        })
                        .collect();

                    for (r, row) in row_data {
                        output
                            .set_row_slice(band, r as isize, &row)
                            .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                        done_rows += 1;
                        compute_progress.emit_unit_fraction(
                            ctx.progress,
                            done_rows as f64 / total_rows as f64,
                        );
                    }

                    row_start = row_end;
                }

                continue;
            }

            if matches!(op, RankOp::Diversity) {
                // Legacy-style running diversity: maintain a quantized histogram while
                // sliding the window left-to-right and update the unique-count incrementally.
                let mut band_min = f64::INFINITY;
                let mut band_max = f64::NEG_INFINITY;
                for r in 0..rows {
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        if z < band_min {
                            band_min = z;
                        }
                        if z > band_max {
                            band_max = z;
                        }
                    }
                }

                if !band_min.is_finite() || !band_max.is_finite() {
                    done_rows += rows;
                    compute_progress.emit_unit_fraction(
                        ctx.progress,
                        done_rows as f64 / total_rows as f64,
                    );
                    continue;
                }

                let multiplier_diversity = if band_min.floor() != band_min || band_max.floor() != band_max {
                    1000.0
                } else {
                    1.0
                };
                let min_bin = (band_min * multiplier_diversity).floor() as i64;
                let max_bin = (band_max * multiplier_diversity).floor() as i64;
                let num_bins_i64 = (max_bin - min_bin + 1).max(1);
                let num_bins = usize::try_from(num_bins_i64).map_err(|_| {
                    ToolError::Execution(
                        "diversity filter histogram bin count exceeds platform limits".to_string(),
                    )
                })?;

                let bin_nodata = i64::MIN;
                let mut binned = vec![bin_nodata; rows * cols];
                binned
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(r, row_bins)| {
                        for (c, cell_bin) in row_bins.iter_mut().enumerate() {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            *cell_bin = (z * multiplier_diversity).floor() as i64 - min_bin;
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

                let mut row_start = 0usize;
                while row_start < rows {
                    let row_end = (row_start + RANK_FILTER_PAR_ROW_BATCH).min(rows);
                    let row_data: Vec<(usize, Vec<f64>)> = (row_start..row_end)
                        .into_par_iter()
                        .map(|r| {
                            let row = r as isize;
                            let mut row_out = vec![nodata; cols];
                            let mut histo = vec![0i64; num_bins];
                            let mut initialized = false;
                            let mut diversity = 0i64;
                            let start_row = row - half_y;
                            let end_row = row + half_y;

                            for c in 0..cols {
                                let col = c as isize;
                                let center_bin = get_bin(row, col);
                                if center_bin == bin_nodata {
                                    initialized = false;
                                    continue;
                                }

                                if initialized {
                                    let trailing_col = col - half_x - 1;
                                    let leading_col = col + half_x;

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, trailing_col);
                                        if bv != bin_nodata {
                                            let bin = &mut histo[bv as usize];
                                            *bin -= 1;
                                            if *bin == 0 {
                                                diversity -= 1;
                                            }
                                        }
                                    }

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, leading_col);
                                        if bv != bin_nodata {
                                            let bin = &mut histo[bv as usize];
                                            if *bin == 0 {
                                                diversity += 1;
                                            }
                                            *bin += 1;
                                        }
                                    }
                                } else {
                                    histo.fill(0);
                                    diversity = 0;
                                    let start_col = col - half_x;
                                    let end_col = col + half_x;

                                    for cc in start_col..=end_col {
                                        for rr in start_row..=end_row {
                                            let bv = get_bin(rr, cc);
                                            if bv != bin_nodata {
                                                let bin = &mut histo[bv as usize];
                                                if *bin == 0 {
                                                    diversity += 1;
                                                }
                                                *bin += 1;
                                            }
                                        }
                                    }
                                }

                                row_out[c] = diversity as f64;
                                initialized = true;
                            }

                            (r, row_out)
                        })
                        .collect();

                    for (r, row) in row_data {
                        output
                            .set_row_slice(band, r as isize, &row)
                            .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                        done_rows += 1;
                        compute_progress.emit_unit_fraction(
                            ctx.progress,
                            done_rows as f64 / total_rows as f64,
                        );
                    }

                    row_start = row_end;
                }

                continue;
            }

            if matches!(op, RankOp::Majority) {
                // Legacy-style running majority: maintain a quantized histogram while
                // sliding the window and update mode tracking incrementally.
                let mut band_min = f64::INFINITY;
                let mut band_max = f64::NEG_INFINITY;
                for r in 0..rows {
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        if z < band_min {
                            band_min = z;
                        }
                        if z > band_max {
                            band_max = z;
                        }
                    }
                }

                if !band_min.is_finite() || !band_max.is_finite() {
                    done_rows += rows;
                    compute_progress.emit_unit_fraction(
                        ctx.progress,
                        done_rows as f64 / total_rows as f64,
                    );
                    continue;
                }

                let multiplier_majority = if band_min.floor() != band_min || band_max.floor() != band_max {
                    100.0
                } else {
                    1.0
                };
                let min_bin = (band_min * multiplier_majority).floor() as i64;
                let max_bin = (band_max * multiplier_majority).floor() as i64;
                let num_bins_i64 = (max_bin - min_bin + 1).max(1);
                let num_bins = usize::try_from(num_bins_i64).map_err(|_| {
                    ToolError::Execution(
                        "majority filter histogram bin count exceeds platform limits".to_string(),
                    )
                })?;

                let bin_nodata = i64::MIN;
                let mut binned = vec![bin_nodata; rows * cols];
                binned
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(r, row_bins)| {
                        for (c, cell_bin) in row_bins.iter_mut().enumerate() {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            *cell_bin = (z * multiplier_majority).floor() as i64 - min_bin;
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

                let mut row_start = 0usize;
                while row_start < rows {
                    let row_end = (row_start + RANK_FILTER_PAR_ROW_BATCH).min(rows);
                    let row_data: Vec<(usize, Vec<f64>)> = (row_start..row_end)
                        .into_par_iter()
                        .map(|r| {
                            let row = r as isize;
                            let mut row_out = vec![nodata; cols];
                            let mut histo = vec![0i64; num_bins];
                            let mut active_bins = HashSet::<usize>::new();
                            let mut initialized = false;
                            let mut mode_bin = 0i64;
                            let mut mode_freq = 0i64;
                            let start_row = row - half_y;
                            let end_row = row + half_y;

                            for c in 0..cols {
                                let col = c as isize;
                                let center_bin = get_bin(row, col);
                                if center_bin == bin_nodata {
                                    initialized = false;
                                    continue;
                                }

                                if initialized {
                                    let trailing_col = col - half_x - 1;
                                    let leading_col = col + half_x;

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, trailing_col);
                                        if bv != bin_nodata {
                                            let idx = bv as usize;
                                            histo[idx] -= 1;
                                            if histo[idx] == 0 {
                                                active_bins.remove(&idx);
                                            }
                                        }
                                    }

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, leading_col);
                                        if bv != bin_nodata {
                                            let idx = bv as usize;
                                            histo[idx] += 1;
                                            if histo[idx] == 1 {
                                                active_bins.insert(idx);
                                            }
                                            if histo[idx] > mode_freq {
                                                mode_freq = histo[idx];
                                                mode_bin = bv;
                                            }
                                        }
                                    }

                                    if mode_bin >= 0 && mode_bin < num_bins_i64 && histo[mode_bin as usize] < mode_freq {
                                        mode_freq = if mode_bin >= 0 && mode_bin < num_bins_i64 {
                                            histo[mode_bin as usize]
                                        } else {
                                            0
                                        };
                                        for &idx in &active_bins {
                                            if histo[idx] > mode_freq {
                                                mode_freq = histo[idx];
                                                mode_bin = idx as i64;
                                            }
                                        }
                                    }
                                } else {
                                    histo.fill(0);
                                    active_bins.clear();
                                    mode_bin = 0;
                                    mode_freq = 0;
                                    let start_col = col - half_x;
                                    let end_col = col + half_x;

                                    for cc in start_col..=end_col {
                                        for rr in start_row..=end_row {
                                            let bv = get_bin(rr, cc);
                                            if bv != bin_nodata {
                                                let idx = bv as usize;
                                                histo[idx] += 1;
                                                if histo[idx] == 1 {
                                                    active_bins.insert(idx);
                                                }
                                                if histo[idx] > mode_freq {
                                                    mode_freq = histo[idx];
                                                    mode_bin = bv;
                                                }
                                            }
                                        }
                                    }
                                }

                                if mode_freq > 0 {
                                    row_out[c] = (mode_bin + min_bin) as f64 / multiplier_majority;
                                }
                                initialized = true;
                            }

                            (r, row_out)
                        })
                        .collect();

                    for (r, row) in row_data {
                        output
                            .set_row_slice(band, r as isize, &row)
                            .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                        done_rows += 1;
                        compute_progress.emit_unit_fraction(
                            ctx.progress,
                            done_rows as f64 / total_rows as f64,
                        );
                    }

                    row_start = row_end;
                }

                continue;
            }

            let mut row_start = 0usize;
            while row_start < rows {
                let row_end = (row_start + RANK_FILTER_PAR_ROW_BATCH).min(rows);
                let row_data: Vec<(usize, Vec<f64>)> = (row_start..row_end)
                    .into_par_iter()
                    .map(|r| {
                        let mut row_out = vec![nodata; cols];
                        let mut bins = Vec::<i64>::with_capacity(filter_x * filter_y);

                        for c in 0..cols {
                            let center = input.get(band, r as isize, c as isize);
                            if input.is_nodata(center) {
                                continue;
                            }

                            bins.clear();
                            let center_bin_rank = if matches!(op, RankOp::Percentile) {
                                Some((center * multiplier_rank).floor() as i64)
                            } else {
                                None
                            };
                            let mut count = 0usize;
                            let mut n_less = 0usize;

                            for ny in (r as isize - half_y)..=(r as isize + half_y) {
                                for nx in (c as isize - half_x)..=(c as isize + half_x) {
                                    let z = input.get(band, ny, nx);
                                    if input.is_nodata(z) {
                                        continue;
                                    }
                                    match op {
                                        RankOp::Median => {
                                            bins.push((z * multiplier_rank).floor() as i64);
                                        }
                                        RankOp::Percentile => {
                                            let q = (z * multiplier_rank).floor() as i64;
                                            if q < center_bin_rank.unwrap() {
                                                n_less += 1;
                                            }
                                            count += 1;
                                        }
                                        RankOp::Majority => {
                                            bins.push((z * 100.0).floor() as i64);
                                        }
                                        RankOp::Diversity => {
                                            bins.push((z * 1000.0).floor() as i64);
                                        }
                                    }
                                }
                            }

                            if (matches!(op, RankOp::Percentile) && count == 0)
                                || (!matches!(op, RankOp::Percentile) && bins.is_empty())
                            {
                                row_out[c] = 0.0;
                                continue;
                            }

                            row_out[c] = match op {
                                RankOp::Median => {
                                    bins.sort_unstable();
                                    bins[bins.len() / 2] as f64 / multiplier_rank
                                }
                                RankOp::Percentile => {
                                    n_less as f64 / count as f64 * 100.0
                                }
                                RankOp::Majority => {
                                    bins.sort_unstable();
                                    let mut mode_bin = bins[0];
                                    let mut mode_freq = 1usize;
                                    let mut run_bin = bins[0];
                                    let mut run_freq = 1usize;

                                    for &bin in bins.iter().skip(1) {
                                        if bin == run_bin {
                                            run_freq += 1;
                                        } else {
                                            if run_freq > mode_freq {
                                                mode_freq = run_freq;
                                                mode_bin = run_bin;
                                            }
                                            run_bin = bin;
                                            run_freq = 1;
                                        }
                                    }

                                    if run_freq > mode_freq {
                                        mode_bin = run_bin;
                                    }

                                    mode_bin as f64 / 100.0
                                }
                                RankOp::Diversity => {
                                    bins.sort_unstable();
                                    let mut unique = 1usize;
                                    let mut prev = bins[0];
                                    for &bin in bins.iter().skip(1) {
                                        if bin != prev {
                                            unique += 1;
                                            prev = bin;
                                        }
                                    }
                                    unique as f64
                                }
                            };
                        }

                        (r, row_out)
                    })
                    .collect();

                for (r, row) in row_data {
                    output
                        .set_row_slice(band, r as isize, &row)
                        .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                    done_rows += 1;
                    compute_progress.emit_unit_fraction(
                        ctx.progress,
                        done_rows as f64 / total_rows as f64,
                    );
                }

                row_start = row_end;
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

macro_rules! define_rank_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                MedianFilterTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                MedianFilterTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = MedianFilterTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                let _ = MedianFilterTool::parse_window_sizes(args);
                let _ = MedianFilterTool::parse_sig_digits(args);
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                MedianFilterTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_rank_tool!(MedianFilterTool, RankOp::Median);
define_rank_tool!(PercentileFilterTool, RankOp::Percentile);
define_rank_tool!(MajorityFilterTool, RankOp::Majority);
define_rank_tool!(DiversityFilterTool, RankOp::Diversity);

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
    fn median_and_majority_constant_raster_equal_input_value() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(5));
        args.insert("filter_size_y".to_string(), json!(5));
        args.insert("sig_digits".to_string(), json!(2));

        let med = run_with_memory(&MedianFilterTool, &mut args.clone(), make_constant_raster(20, 20, 7.0));
        let maj = run_with_memory(&MajorityFilterTool, &mut args, make_constant_raster(20, 20, 7.0));
        assert!((med.get(0, 10, 10) - 7.0).abs() < 1e-9);
        assert!((maj.get(0, 10, 10) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn percentile_and_diversity_constant_raster_expected_values() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(5));
        args.insert("filter_size_y".to_string(), json!(5));
        args.insert("sig_digits".to_string(), json!(2));

        let pct = run_with_memory(&PercentileFilterTool, &mut args.clone(), make_constant_raster(20, 20, 5.0));
        let div = run_with_memory(&DiversityFilterTool, &mut args, make_constant_raster(20, 20, 5.0));
        assert!(pct.get(0, 10, 10).abs() < 1e-9);
        assert!((div.get(0, 10, 10) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn median_filter_progress_is_monotonic_bounded_and_completes() {
        let input = make_constant_raster(1024, 1024, 7.0);
        let input_id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&input_id)),
        );
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("sig_digits".to_string(), json!(2));

        let caps = AllowAllCapabilities;
        let progress = RecordingProgress::new();
        let ctx = ToolContext {
            progress: &progress,
            capabilities: &caps,
        };

        let tool = MedianFilterTool;
        let _ = tool.run(&args, &ctx).expect("median filter should run");

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
