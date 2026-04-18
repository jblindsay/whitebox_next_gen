use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, PercentCoalescer, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterConfig, RasterFormat};

use crate::memory_store;

pub struct GlcmTextureTool;

#[derive(Clone, Copy, PartialEq, Eq)]
enum GlcmFeature {
    Contrast,
    Dissimilarity,
    Homogeneity,
    Asm,
    Energy,
    Entropy,
    Mean,
    Variance,
    Correlation,
}

impl GlcmFeature {
    fn id(self) -> &'static str {
        match self {
            Self::Contrast => "contrast",
            Self::Dissimilarity => "dissimilarity",
            Self::Homogeneity => "homogeneity",
            Self::Asm => "asm",
            Self::Energy => "energy",
            Self::Entropy => "entropy",
            Self::Mean => "mean",
            Self::Variance => "variance",
            Self::Correlation => "correlation",
        }
    }

}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirectionAggregation {
    Mean,
    Min,
    Max,
    Range,
    Separate,
}

impl DirectionAggregation {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "mean" => Some(Self::Mean),
            "min" => Some(Self::Min),
            "max" => Some(Self::Max),
            "range" => Some(Self::Range),
            "separate" => Some(Self::Separate),
            _ => None,
        }
    }
}

fn parse_csv_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_features(raw: &str) -> Result<Vec<GlcmFeature>, ToolError> {
    let tokens = parse_csv_list(raw);
    if tokens.is_empty() {
        return Err(ToolError::Validation(
            "parameter 'features' must contain at least one feature name".to_string(),
        ));
    }

    let mut out = Vec::new();
    for t in tokens {
        let f = match t.as_str() {
            "contrast" => GlcmFeature::Contrast,
            "dissimilarity" => GlcmFeature::Dissimilarity,
            "homogeneity" => GlcmFeature::Homogeneity,
            "asm" => GlcmFeature::Asm,
            "energy" => GlcmFeature::Energy,
            "entropy" => GlcmFeature::Entropy,
            "mean" => GlcmFeature::Mean,
            "variance" => GlcmFeature::Variance,
            "correlation" => GlcmFeature::Correlation,
            _ => {
                return Err(ToolError::Validation(format!(
                    "unknown GLCM feature '{}'; supported: contrast, dissimilarity, homogeneity, asm, energy, entropy, mean, variance, correlation",
                    t
                )));
            }
        };
        if !out.contains(&f) {
            out.push(f);
        }
    }

    Ok(out)
}

fn parse_angles(raw: &str) -> Result<Vec<i32>, ToolError> {
    let tokens = parse_csv_list(raw);
    if tokens.is_empty() {
        return Err(ToolError::Validation(
            "parameter 'angles' must contain at least one angle (0,45,90,135)".to_string(),
        ));
    }

    let mut out = Vec::new();
    for t in tokens {
        let mut angle = t.parse::<i32>().map_err(|_| {
            ToolError::Validation(format!(
                "invalid angle '{}'; supported angles are 0,45,90,135",
                t
            ))
        })?;
        angle = ((angle % 180) + 180) % 180;
        if !matches!(angle, 0 | 45 | 90 | 135) {
            return Err(ToolError::Validation(format!(
                "unsupported angle '{}'; supported angles are 0,45,90,135",
                angle
            )));
        }
        if !out.contains(&angle) {
            out.push(angle);
        }
    }

    Ok(out)
}

fn angle_to_offset(angle: i32, distance: isize) -> (isize, isize) {
    match angle {
        0 => (0, distance),
        45 => (-distance, distance),
        90 => (-distance, 0),
        135 => (-distance, -distance),
        _ => (0, distance),
    }
}

fn raster_index(cols: usize, row: isize, col: isize) -> usize {
    (row as usize) * cols + (col as usize)
}

fn quantize_input(raster: &Raster, levels: usize) -> Result<Vec<i32>, ToolError> {
    let rows = raster.rows as isize;
    let cols = raster.cols as isize;

    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for row in 0..rows {
        for col in 0..cols {
            let z = raster.get(0, row, col);
            if raster.is_nodata(z) {
                continue;
            }
            min_v = min_v.min(z);
            max_v = max_v.max(z);
        }
    }

    if !min_v.is_finite() || !max_v.is_finite() {
        return Err(ToolError::Validation(
            "input raster contains no valid cells".to_string(),
        ));
    }

    let mut out = vec![-1i32; raster.rows * raster.cols];
    let range = (max_v - min_v).max(1e-12);
    for row in 0..rows {
        for col in 0..cols {
            let z = raster.get(0, row, col);
            let idx = raster_index(raster.cols, row, col);
            if raster.is_nodata(z) {
                out[idx] = -1;
                continue;
            }
            let q = (((z - min_v) / range) * (levels as f64 - 1.0)).round();
            out[idx] = q.clamp(0.0, levels as f64 - 1.0) as i32;
        }
    }

    Ok(out)
}

fn compute_glcm_metrics(
    quantized: &[i32],
    rows: isize,
    cols: isize,
    center_row: isize,
    center_col: isize,
    half_window: isize,
    dr: isize,
    dc: isize,
    levels: usize,
    symmetric: bool,
) -> Option<[f64; 9]> {
    let mut matrix = vec![0.0f64; levels * levels];
    let mut total = 0.0f64;

    let r0 = (center_row - half_window).max(0);
    let r1 = (center_row + half_window).min(rows - 1);
    let c0 = (center_col - half_window).max(0);
    let c1 = (center_col + half_window).min(cols - 1);

    for row in r0..=r1 {
        for col in c0..=c1 {
            let nr = row + dr;
            let nc = col + dc;
            if nr < r0 || nr > r1 || nc < c0 || nc > c1 {
                continue;
            }
            if nr < 0 || nr >= rows || nc < 0 || nc >= cols {
                continue;
            }

            let q1 = quantized[raster_index(cols as usize, row, col)];
            let q2 = quantized[raster_index(cols as usize, nr, nc)];
            if q1 < 0 || q2 < 0 {
                continue;
            }

            let i = q1 as usize;
            let j = q2 as usize;
            matrix[i * levels + j] += 1.0;
            total += 1.0;
            if symmetric {
                matrix[j * levels + i] += 1.0;
                total += 1.0;
            }
        }
    }

    if total <= 0.0 {
        return None;
    }

    let mut contrast = 0.0;
    let mut dissimilarity = 0.0;
    let mut homogeneity = 0.0;
    let mut asm = 0.0;
    let mut entropy = 0.0;
    let mut mean_i = 0.0;
    let mut mean_j = 0.0;

    for i in 0..levels {
        for j in 0..levels {
            let p = matrix[i * levels + j] / total;
            if p <= 0.0 {
                continue;
            }
            let diff = (i as f64 - j as f64).abs();
            let diff2 = diff * diff;
            contrast += p * diff2;
            dissimilarity += p * diff;
            homogeneity += p / (1.0 + diff2);
            asm += p * p;
            entropy -= p * p.ln();
            mean_i += (i as f64) * p;
            mean_j += (j as f64) * p;
        }
    }

    let mut var_i = 0.0;
    let mut var_j = 0.0;
    let mut cov = 0.0;
    for i in 0..levels {
        for j in 0..levels {
            let p = matrix[i * levels + j] / total;
            if p <= 0.0 {
                continue;
            }
            let di = i as f64 - mean_i;
            let dj = j as f64 - mean_j;
            var_i += p * di * di;
            var_j += p * dj * dj;
            cov += p * di * dj;
        }
    }

    let correlation = if var_i > 0.0 && var_j > 0.0 {
        cov / (var_i.sqrt() * var_j.sqrt())
    } else {
        0.0
    };

    let energy = asm.sqrt();
    let mean = 0.5 * (mean_i + mean_j);
    let variance = 0.5 * (var_i + var_j);

    Some([
        contrast,
        dissimilarity,
        homogeneity,
        asm,
        energy,
        entropy,
        mean,
        variance,
        correlation,
    ])
}

fn feature_value(metrics: &[f64; 9], feature: GlcmFeature) -> f64 {
    match feature {
        GlcmFeature::Contrast => metrics[0],
        GlcmFeature::Dissimilarity => metrics[1],
        GlcmFeature::Homogeneity => metrics[2],
        GlcmFeature::Asm => metrics[3],
        GlcmFeature::Energy => metrics[4],
        GlcmFeature::Entropy => metrics[5],
        GlcmFeature::Mean => metrics[6],
        GlcmFeature::Variance => metrics[7],
        GlcmFeature::Correlation => metrics[8],
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

fn store_named_raster_output(
    raster: Raster,
    output_path: Option<std::path::PathBuf>,
) -> Result<serde_json::Value, ToolError> {
    let locator = write_or_store_output(raster, output_path)?;
    Ok(json!({"__wbw_type__": "raster", "path": locator, "active_band": 0}))
}

impl Tool for GlcmTextureTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "glcm_texture",
            display_name: "GLCM Texture",
            summary: "Computes general-purpose local GLCM texture rasters as a multiband output.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input single-band raster.", required: true },
                ToolParamSpec { name: "window_size", description: "Odd moving-window size in pixels (default 7).", required: false },
                ToolParamSpec { name: "distance", description: "Pixel distance for the co-occurrence pair offsets (default 1).", required: false },
                ToolParamSpec { name: "angles", description: "Comma-separated angles from 0,45,90,135 (default '0,45,90,135').", required: false },
                ToolParamSpec { name: "features", description: "Comma-separated features: contrast,dissimilarity,homogeneity,asm,energy,entropy,mean,variance,correlation.", required: false },
                ToolParamSpec { name: "direction_aggregation", description: "Direction aggregation mode: mean (default), min, max, range, separate.", required: false },
                ToolParamSpec { name: "levels", description: "Number of quantization levels (default 32; range 8-128).", required: false },
                ToolParamSpec { name: "symmetric", description: "If true (default), build symmetric GLCMs.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path (multiband GeoTIFF).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("window_size".to_string(), json!(7));
        defaults.insert("distance".to_string(), json!(1));
        defaults.insert("angles".to_string(), json!("0,45,90,135"));
        defaults.insert("features".to_string(), json!("contrast,homogeneity,energy,entropy"));
        defaults.insert("direction_aggregation".to_string(), json!("mean"));
        defaults.insert("levels".to_string(), json!(32));
        defaults.insert("symmetric".to_string(), json!(true));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("gray.tif"));
        example.insert("window_size".to_string(), json!(9));
        example.insert("distance".to_string(), json!(1));
        example.insert("angles".to_string(), json!("0,45,90,135"));
        example.insert("features".to_string(), json!("contrast,homogeneity,entropy"));
        example.insert("direction_aggregation".to_string(), json!("mean"));
        example.insert("levels".to_string(), json!(32));
        example.insert("output".to_string(), json!("glcm_texture.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
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
                name: "basic_glcm_texture".to_string(),
                description: "Compute a multiband GLCM texture raster with directional averaging.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "texture".to_string(),
                "glcm".to_string(),
                "raster".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;

        let window_size = args
            .get("window_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(7);
        if window_size < 3 || window_size % 2 == 0 {
            return Err(ToolError::Validation(
                "parameter 'window_size' must be an odd integer >= 3".to_string(),
            ));
        }

        let _ = parse_angles(
            args.get("angles")
                .and_then(|v| v.as_str())
                .unwrap_or("0,45,90,135"),
        )?;

        let _ = parse_features(
            args.get("features")
                .and_then(|v| v.as_str())
                .unwrap_or("contrast,homogeneity,energy,entropy"),
        )?;

        let agg_raw = args
            .get("direction_aggregation")
            .and_then(|v| v.as_str())
            .unwrap_or("mean");
        if DirectionAggregation::parse(agg_raw).is_none() {
            return Err(ToolError::Validation(
                "parameter 'direction_aggregation' must be one of: mean, min, max, range, separate"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = parse_raster_path_arg(args, "input")?;
        let window_size = args
            .get("window_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(7)
            .max(3);
        if window_size % 2 == 0 {
            return Err(ToolError::Validation(
                "parameter 'window_size' must be odd".to_string(),
            ));
        }
        let half_window = (window_size / 2) as isize;
        let distance = args
            .get("distance")
            .and_then(|v| v.as_u64())
            .map(|v| v as isize)
            .unwrap_or(1)
            .max(1);
        let levels = args
            .get("levels")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(32)
            .clamp(8, 128);
        let symmetric = args
            .get("symmetric")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let angles = parse_angles(
            args.get("angles")
                .and_then(|v| v.as_str())
                .unwrap_or("0,45,90,135"),
        )?;
        let features = parse_features(
            args.get("features")
                .and_then(|v| v.as_str())
                .unwrap_or("contrast,homogeneity,energy,entropy"),
        )?;
        let aggregation = DirectionAggregation::parse(
            args.get("direction_aggregation")
                .and_then(|v| v.as_str())
                .unwrap_or("mean"),
        )
        .ok_or_else(|| {
            ToolError::Validation(
                "parameter 'direction_aggregation' must be one of: mean, min, max, range, separate"
                    .to_string(),
            )
        })?;
        let output_path = parse_optional_output_path(args, "output")?;

        let input = Raster::read(&input_path).map_err(|e| {
            ToolError::Execution(format!("failed reading input raster '{}': {}", input_path, e))
        })?;

        let rows = input.rows as isize;
        let cols = input.cols as isize;
        let quantized = quantize_input(&input, levels)?;
        let offsets: Vec<(isize, isize)> = angles
            .iter()
            .map(|a| angle_to_offset(*a, distance))
            .collect();

        let mut band_names = Vec::new();
        if aggregation == DirectionAggregation::Separate {
            for feature in &features {
                for angle in &angles {
                    band_names.push(format!("{}_{}", feature.id(), angle));
                }
            }
        } else {
            for feature in &features {
                band_names.push(format!("{}_{}", feature.id(), args
                    .get("direction_aggregation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("mean")
                    .to_ascii_lowercase()));
            }
        }

        let mut metadata = input.metadata.clone();
        metadata.push(("texture_tool".to_string(), "glcm_texture".to_string()));
        metadata.push(("glcm_features".to_string(), features.iter().map(|f| f.id()).collect::<Vec<_>>().join(",")));
        metadata.push(("glcm_angles".to_string(), angles.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(",")));
        metadata.push(("glcm_levels".to_string(), levels.to_string()));
        metadata.push(("glcm_direction_aggregation".to_string(), args
            .get("direction_aggregation")
            .and_then(|v| v.as_str())
            .unwrap_or("mean")
            .to_string()));

        let mut output = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: band_names.len().max(1),
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: input.crs.clone(),
            metadata,
        });

        for (i, name) in band_names.iter().enumerate() {
            output
                .metadata
                .push((format!("band_{}_name", i + 1), name.to_string()));
        }

        let nodata = -32768.0;
        let rows_usize = rows as usize;
        let cols_usize = cols as usize;
        let band_count = band_names.len();

        // Compute each row in parallel, then write outputs in a single thread.
        let row_results: Vec<Vec<f64>> = (0..rows_usize)
            .into_par_iter()
            .map(|row_u| {
                let row = row_u as isize;
                let mut row_output = vec![nodata; band_count * cols_usize];

                for col_u in 0..cols_usize {
                    let col = col_u as isize;
                    let center_idx = raster_index(input.cols, row, col);
                    if quantized[center_idx] < 0 {
                        continue;
                    }

                    let mut values_per_angle: Vec<Vec<f64>> = Vec::new();
                    for (dr, dc) in &offsets {
                        if let Some(metrics) = compute_glcm_metrics(
                            &quantized,
                            rows,
                            cols,
                            row,
                            col,
                            half_window,
                            *dr,
                            *dc,
                            levels,
                            symmetric,
                        ) {
                            let mut vals = Vec::with_capacity(features.len());
                            for f in &features {
                                vals.push(feature_value(&metrics, *f));
                            }
                            values_per_angle.push(vals);
                        }
                    }

                    if values_per_angle.is_empty() {
                        continue;
                    }

                    if aggregation == DirectionAggregation::Separate {
                        let mut band = 0usize;
                        for fi in 0..features.len() {
                            for ai in 0..angles.len() {
                                let z = if ai < values_per_angle.len() {
                                    values_per_angle[ai][fi]
                                } else {
                                    nodata
                                };
                                row_output[band * cols_usize + col_u] = z;
                                band += 1;
                            }
                        }
                    } else {
                        for fi in 0..features.len() {
                            let mut min_v = f64::INFINITY;
                            let mut max_v = f64::NEG_INFINITY;
                            let mut sum_v = 0.0;
                            for row_vals in &values_per_angle {
                                let v = row_vals[fi];
                                min_v = min_v.min(v);
                                max_v = max_v.max(v);
                                sum_v += v;
                            }
                            let z = match aggregation {
                                DirectionAggregation::Mean => sum_v / values_per_angle.len() as f64,
                                DirectionAggregation::Min => min_v,
                                DirectionAggregation::Max => max_v,
                                DirectionAggregation::Range => max_v - min_v,
                                DirectionAggregation::Separate => nodata,
                            };
                            row_output[fi * cols_usize + col_u] = z;
                        }
                    }
                }

                row_output
            })
            .collect();

        for (row_u, row_output) in row_results.iter().enumerate() {
            let row = row_u as isize;
            for b in 0..band_count {
                let band_offset = b * cols_usize;
                for col_u in 0..cols_usize {
                    let z = row_output[band_offset + col_u];
                    let _ = output.set(b as isize, row, col_u as isize, z);
                }
            }

            if row_u % 10 == 0 {
                coalescer.emit_unit_fraction(ctx.progress, (row_u as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }

        ctx.progress.progress(1.0);

        let raster_ref = store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_ref);
        Ok(ToolRunResult { outputs })
    }
}
