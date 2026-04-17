use super::non_filter_tools::{
    GeneralizeClassifiedRasterTool, ImageSegmentationTool,
};
use smartcore::ensemble::random_forest_classifier::{
    RandomForestClassifier, RandomForestClassifierParameters,
};
use smartcore::linalg::basic::matrix::DenseMatrix;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use wbcore::*;
use wbraster::Raster;

fn parse_raster_list_arg(args: &ToolArgs, name: &str) -> Result<Vec<String>, ToolError> {
    let value = args
        .get(name)
        .ok_or_else(|| ToolError::Validation(format!("missing required parameter '{name}'")))?;
    let arr = value
        .as_array()
        .ok_or_else(|| ToolError::Validation(format!("parameter '{name}' must be an array of raster paths")))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(s) = item.as_str() else {
            return Err(ToolError::Validation(format!(
                "parameter '{name}' must contain only string raster paths"
            )));
        };
        out.push(s.to_string());
    }
    if out.is_empty() {
        return Err(ToolError::Validation(format!(
            "parameter '{name}' must contain at least one raster path"
        )));
    }
    Ok(out)
}

fn parse_required_path_arg(args: &ToolArgs, name: &str) -> Result<String, ToolError> {
    args.get(name)
        .and_then(serde_json::Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::Validation(format!("missing required parameter '{name}'")))
}

fn parse_optional_path_arg(args: &ToolArgs, name: &str) -> Option<String> {
    args.get(name)
        .and_then(serde_json::Value::as_str)
        .map(|s| s.to_string())
}

fn parse_usize_arg(args: &ToolArgs, name: &str, default_value: usize) -> usize {
    args.get(name)
        .and_then(serde_json::Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(default_value)
}

fn parse_f64_arg(args: &ToolArgs, name: &str, default_value: f64) -> f64 {
    args.get(name)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(default_value)
}

fn output_path_or_default(input_path: &str, suffix: &str, ext: &str, explicit: Option<String>) -> String {
    if let Some(path) = explicit {
        return path;
    }
    let p = Path::new(input_path);
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    parent
        .join(format!("{stem}_{suffix}.{ext}"))
        .to_string_lossy()
        .to_string()
}

fn output_csv_path_or_default(base_path: &str, suffix: &str, explicit: Option<String>) -> String {
    output_path_or_default(base_path, suffix, "csv", explicit)
}

fn result_path_from_outputs(outputs: &BTreeMap<String, serde_json::Value>) -> Option<String> {
    outputs
        .get("path")
        .and_then(serde_json::Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| {
            outputs
                .get("output")
                .and_then(serde_json::Value::as_str)
                .map(|s| s.to_string())
        })
}

fn find_header_index(headers: &[String], name: &str) -> Result<usize, ToolError> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| ToolError::Validation(format!("column '{name}' not found")))
}

fn parse_simple_csv(path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), ToolError> {
    let file = File::open(path)
        .map_err(|e| ToolError::Execution(format!("failed to open CSV '{path}': {e}")))?;
    let reader = BufReader::new(file);

    let mut lines = reader.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| ToolError::Validation(format!("CSV '{path}' is empty")))
        .and_then(|l| l.map_err(|e| ToolError::Execution(format!("failed reading CSV header: {e}"))))?;

    let headers: Vec<String> = header_line
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    if headers.is_empty() {
        return Err(ToolError::Validation(format!(
            "CSV '{path}' has no header columns"
        )));
    }

    let mut rows = Vec::new();
    for line in lines {
        let line =
            line.map_err(|e| ToolError::Execution(format!("failed reading CSV '{path}': {e}")))?;
        if line.trim().is_empty() {
            continue;
        }
        let row: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
        if row.len() != headers.len() {
            return Err(ToolError::Validation(format!(
                "CSV '{path}' row has {} columns but header has {}",
                row.len(),
                headers.len()
            )));
        }
        rows.push(row);
    }
    Ok((headers, rows))
}

fn write_csv(path: &str, header: &[String], rows: &[Vec<String>]) -> Result<(), ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ToolError::Execution(format!(
                "failed creating output directory '{}': {e}",
                parent.display()
            ))
        })?;
    }

    let mut file = File::create(path)
        .map_err(|e| ToolError::Execution(format!("failed creating CSV '{path}': {e}")))?;

    writeln!(file, "{}", header.join(","))
        .map_err(|e| ToolError::Execution(format!("failed writing CSV header: {e}")))?;

    for row in rows {
        writeln!(file, "{}", row.join(","))
            .map_err(|e| ToolError::Execution(format!("failed writing CSV row: {e}")))?;
    }

    Ok(())
}

#[derive(Clone)]
struct SpectralStats {
    count: usize,
    sum: Vec<f64>,
    sumsq: Vec<f64>,
    min: Vec<f64>,
    max: Vec<f64>,
}

impl SpectralStats {
    fn new(bands: usize) -> Self {
        Self {
            count: 0,
            sum: vec![0.0; bands],
            sumsq: vec![0.0; bands],
            min: vec![f64::INFINITY; bands],
            max: vec![-f64::INFINITY; bands],
        }
    }

    fn update(&mut self, values: &[f64]) {
        self.count += 1;
        for (i, &v) in values.iter().enumerate() {
            self.sum[i] += v;
            self.sumsq[i] += v * v;
            if v < self.min[i] {
                self.min[i] = v;
            }
            if v > self.max[i] {
                self.max[i] = v;
            }
        }
    }
}

#[derive(Clone, Default)]
struct ShapeStats {
    area_px: usize,
    perimeter_edges: usize,
    min_row: isize,
    max_row: isize,
    min_col: isize,
    max_col: isize,
    initialized: bool,
}

impl ShapeStats {
    fn update_cell(&mut self, row: isize, col: isize) {
        self.area_px += 1;
        if !self.initialized {
            self.min_row = row;
            self.max_row = row;
            self.min_col = col;
            self.max_col = col;
            self.initialized = true;
            return;
        }
        self.min_row = self.min_row.min(row);
        self.max_row = self.max_row.max(row);
        self.min_col = self.min_col.min(col);
        self.max_col = self.max_col.max(col);
    }
}

pub struct SegmentSlicSuperpixelsTool;

impl Tool for SegmentSlicSuperpixelsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "segment_slic_superpixels",
            display_name: "Segment SLIC Superpixels",
            summary: "Produces compact superpixel-like segments from a multi-band stack (open-core OBIA baseline).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "region_size", description: "Target superpixel size in pixels (default 20).", required: false },
                ToolParamSpec { name: "compactness", description: "Compactness control (default 10.0).", required: false },
                ToolParamSpec { name: "min_area", description: "Minimum area for cleanup merge (default derived from region_size).", required: false },
                ToolParamSpec { name: "output", description: "Optional output segments raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), serde_json::json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("region_size".to_string(), serde_json::json!(20));
        defaults.insert("compactness".to_string(), serde_json::json!(10.0));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "segment_slic_baseline".to_string(),
                description: "Generate compact open-core baseline segments for OBIA workflows.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("inputs".to_string(), serde_json::json!(["red.tif", "green.tif", "nir.tif"]));
                    a.insert("region_size".to_string(), serde_json::json!(18));
                    a.insert("compactness".to_string(), serde_json::json!(12.0));
                    a.insert("output".to_string(), serde_json::json!("segments_slic.tif"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "segmentation".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_list_arg(args, "inputs")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        let region_size = parse_usize_arg(args, "region_size", 20).max(4);
        let compactness = parse_f64_arg(args, "compactness", 10.0).max(0.1);
        let min_area = parse_usize_arg(args, "min_area", (region_size * region_size) / 4).max(1);

        // Reuse the existing robust seeded-region-growing implementation as the
        // first open-core OBIA segmentation baseline.
        let mut delegated = ToolArgs::new();
        delegated.insert("inputs".to_string(), serde_json::json!(inputs));
        let threshold = (0.20 + compactness / 50.0).clamp(0.2, 2.0);
        delegated.insert("threshold".to_string(), serde_json::json!(threshold));
        delegated.insert("steps".to_string(), serde_json::json!(10));
        delegated.insert("min_area".to_string(), serde_json::json!(min_area));
        if let Some(output) = parse_optional_path_arg(args, "output") {
            delegated.insert("output".to_string(), serde_json::json!(output));
        }

        ImageSegmentationTool.run(&delegated, ctx)
    }
}

pub struct SegmentGraphFelzenszwalbTool;

impl Tool for SegmentGraphFelzenszwalbTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "segment_graph_felzenszwalb",
            display_name: "Segment Graph Felzenszwalb",
            summary: "Graph-style segmentation baseline for OBIA (open-core), mapped to robust existing segmentation primitives.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "k", description: "Segmentation scale parameter (default 500.0).", required: false },
                ToolParamSpec { name: "sigma", description: "Optional smoothing hint (default 0.8).", required: false },
                ToolParamSpec { name: "min_area", description: "Minimum area for post-merge cleanup (default 20).", required: false },
                ToolParamSpec { name: "output", description: "Optional output segments raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("inputs".to_string(), serde_json::json!(["band1.tif", "band2.tif", "band3.tif"]));
                d.insert("k".to_string(), serde_json::json!(500.0));
                d.insert("sigma".to_string(), serde_json::json!(0.8));
                d.insert("min_area".to_string(), serde_json::json!(20));
                d
            },
            examples: vec![ToolExample {
                name: "segment_graph_baseline".to_string(),
                description: "Generate graph-style OBIA baseline segments.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("inputs".to_string(), serde_json::json!(["red.tif", "green.tif", "nir.tif"]));
                    a.insert("k".to_string(), serde_json::json!(350.0));
                    a.insert("min_area".to_string(), serde_json::json!(16));
                    a.insert("output".to_string(), serde_json::json!("segments_graph.tif"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "segmentation".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_list_arg(args, "inputs")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        let k = parse_f64_arg(args, "k", 500.0).max(1.0);
        let sigma = parse_f64_arg(args, "sigma", 0.8).max(0.0);
        let min_area = parse_usize_arg(args, "min_area", 20).max(1);

        let mut delegated = ToolArgs::new();
        delegated.insert("inputs".to_string(), serde_json::json!(inputs));

        // Approximate graph-scale behavior via threshold shaping over existing
        // seeded-region-growing segmentation.
        let threshold = ((300.0 / k) + (sigma * 0.25)).clamp(0.1, 2.5);
        delegated.insert("threshold".to_string(), serde_json::json!(threshold));
        delegated.insert("steps".to_string(), serde_json::json!(12));
        delegated.insert("min_area".to_string(), serde_json::json!(min_area));
        if let Some(output) = parse_optional_path_arg(args, "output") {
            delegated.insert("output".to_string(), serde_json::json!(output));
        }

        ImageSegmentationTool.run(&delegated, ctx)
    }
}

pub struct SegmentsMergeSmallRegionsTool;

impl Tool for SegmentsMergeSmallRegionsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "segments_merge_small_regions",
            display_name: "Segments Merge Small Regions",
            summary: "Merges undersized segment regions into neighboring larger regions.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "segments", description: "Input segment-label raster.", required: true },
                ToolParamSpec { name: "min_size", description: "Minimum segment size in cells (default 5).", required: false },
                ToolParamSpec { name: "method", description: "Merge method: longest, largest, nearest (default longest).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("segments".to_string(), serde_json::json!("segments.tif"));
                d.insert("min_size".to_string(), serde_json::json!(5));
                d.insert("method".to_string(), serde_json::json!("longest"));
                d
            },
            examples: vec![ToolExample {
                name: "merge_small_segments".to_string(),
                description: "Remove tiny segment islands while preserving larger boundaries.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("segments".to_string(), serde_json::json!("segments.tif"));
                    a.insert("min_size".to_string(), serde_json::json!(12));
                    a.insert("method".to_string(), serde_json::json!("longest"));
                    a.insert("output".to_string(), serde_json::json!("segments_clean.tif"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "segmentation".to_string(),
                "postprocess".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_path_arg(args, "segments")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let mut delegated = ToolArgs::new();
        delegated.insert(
            "input".to_string(),
            serde_json::json!(parse_required_path_arg(args, "segments")?),
        );
        delegated.insert(
            "min_size".to_string(),
            serde_json::json!(parse_usize_arg(args, "min_size", 5)),
        );
        delegated.insert(
            "method".to_string(),
            serde_json::json!(
                args.get("method")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("longest")
            ),
        );
        if let Some(output) = parse_optional_path_arg(args, "output") {
            delegated.insert("output".to_string(), serde_json::json!(output));
        }

        GeneralizeClassifiedRasterTool.run(&delegated, ctx)
    }
}

pub struct ObjectFeaturesSpectralBasicTool;

impl Tool for ObjectFeaturesSpectralBasicTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "object_features_spectral_basic",
            display_name: "Object Features Spectral Basic",
            summary: "Computes per-segment basic spectral statistics (mean/std/min/max) from input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "segments", description: "Input segment-label raster.", required: true },
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters used for spectral features.", required: true },
                ToolParamSpec { name: "output", description: "Output CSV path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("segments".to_string(), serde_json::json!("segments.tif"));
                d.insert("inputs".to_string(), serde_json::json!(["red.tif", "green.tif", "nir.tif"]));
                d
            },
            examples: vec![ToolExample {
                name: "spectral_features".to_string(),
                description: "Extract basic spectral features for object classification.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("segments".to_string(), serde_json::json!("segments_clean.tif"));
                    a.insert("inputs".to_string(), serde_json::json!(["red.tif", "green.tif", "nir.tif"]));
                    a.insert("output".to_string(), serde_json::json!("object_features_spectral.csv"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "features".to_string(),
                "spectral".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_path_arg(args, "segments")?;
        let _ = parse_raster_list_arg(args, "inputs")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let segments_path = parse_required_path_arg(args, "segments")?;
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let output_path = output_csv_path_or_default(
            &segments_path,
            "object_features_spectral",
            parse_optional_path_arg(args, "output"),
        );

        let segments = Raster::read(&segments_path)
            .map_err(|e| ToolError::Execution(format!("failed reading segments raster: {e}")))?;
        if segments.bands != 1 {
            return Err(ToolError::Validation(
                "segments raster must be single-band".to_string(),
            ));
        }

        let mut rasters = Vec::with_capacity(input_paths.len());
        for path in &input_paths {
            let r = Raster::read(path)
                .map_err(|e| ToolError::Execution(format!("failed reading input raster '{path}': {e}")))?;
            if r.rows != segments.rows || r.cols != segments.cols {
                return Err(ToolError::Validation(format!(
                    "input raster '{path}' dimensions do not match segments raster"
                )));
            }
            rasters.push(r);
        }

        let rows = segments.rows as isize;
        let cols = segments.cols as isize;
        let band_count = rasters.len();

        let mut stats: HashMap<i64, SpectralStats> = HashMap::new();

        for row in 0..rows {
            for col in 0..cols {
                let seg_val = segments.get(0, row, col);
                if segments.is_nodata(seg_val) || seg_val <= 0.0 {
                    continue;
                }
                let seg_id = seg_val.round() as i64;

                let mut values = vec![0.0; band_count];
                let mut valid = true;
                for (i, r) in rasters.iter().enumerate() {
                    let z = r.get(0, row, col);
                    if r.is_nodata(z) {
                        valid = false;
                        break;
                    }
                    values[i] = z;
                }
                if !valid {
                    continue;
                }

                stats
                    .entry(seg_id)
                    .or_insert_with(|| SpectralStats::new(band_count))
                    .update(&values);
            }
        }

        let mut header = vec!["segment_id".to_string(), "count".to_string()];
        for i in 0..band_count {
            header.push(format!("mean_b{}", i + 1));
            header.push(format!("std_b{}", i + 1));
            header.push(format!("min_b{}", i + 1));
            header.push(format!("max_b{}", i + 1));
        }

        let mut ids: Vec<i64> = stats.keys().copied().collect();
        ids.sort_unstable();

        let mut rows_out = Vec::with_capacity(ids.len());
        for seg_id in ids {
            let s = stats.get(&seg_id).expect("segment stats must exist");
            let mut row = vec![seg_id.to_string(), s.count.to_string()];
            for i in 0..band_count {
                let mean = if s.count > 0 {
                    s.sum[i] / s.count as f64
                } else {
                    0.0
                };
                let var = if s.count > 1 {
                    (s.sumsq[i] / s.count as f64) - (mean * mean)
                } else {
                    0.0
                }
                .max(0.0);
                let std = var.sqrt();
                row.push(mean.to_string());
                row.push(std.to_string());
                row.push(s.min[i].to_string());
                row.push(s.max[i].to_string());
            }
            rows_out.push(row);
        }

        write_csv(&output_path, &header, &rows_out)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), serde_json::json!(output_path));
        Ok(ToolRunResult { outputs })
    }
}

pub struct ObjectFeaturesShapeBasicTool;

impl Tool for ObjectFeaturesShapeBasicTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "object_features_shape_basic",
            display_name: "Object Features Shape Basic",
            summary: "Computes per-segment basic shape attributes (area, perimeter, compactness, elongation).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "segments", description: "Input segment-label raster.", required: true },
                ToolParamSpec { name: "output", description: "Output CSV path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("segments".to_string(), serde_json::json!("segments.tif"));
                d
            },
            examples: vec![ToolExample {
                name: "shape_features".to_string(),
                description: "Extract area/perimeter/compactness for OBIA segments.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("segments".to_string(), serde_json::json!("segments_clean.tif"));
                    a.insert("output".to_string(), serde_json::json!("object_features_shape.csv"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "features".to_string(),
                "shape".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_path_arg(args, "segments")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let segments_path = parse_required_path_arg(args, "segments")?;
        let output_path = output_csv_path_or_default(
            &segments_path,
            "object_features_shape",
            parse_optional_path_arg(args, "output"),
        );

        let segments = Raster::read(&segments_path)
            .map_err(|e| ToolError::Execution(format!("failed reading segments raster: {e}")))?;
        if segments.bands != 1 {
            return Err(ToolError::Validation(
                "segments raster must be single-band".to_string(),
            ));
        }

        let rows = segments.rows as isize;
        let cols = segments.cols as isize;

        let mut stats: HashMap<i64, ShapeStats> = HashMap::new();

        let n4 = [(0isize, 1isize), (1, 0), (0, -1), (-1, 0)];

        for row in 0..rows {
            for col in 0..cols {
                let seg_val = segments.get(0, row, col);
                if segments.is_nodata(seg_val) || seg_val <= 0.0 {
                    continue;
                }
                let seg_id = seg_val.round() as i64;

                let entry = stats.entry(seg_id).or_default();
                entry.update_cell(row, col);

                for (dr, dc) in n4 {
                    let nr = row + dr;
                    let nc = col + dc;
                    if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                        entry.perimeter_edges += 1;
                        continue;
                    }
                    let n_val = segments.get(0, nr, nc);
                    if segments.is_nodata(n_val) || n_val.round() as i64 != seg_id {
                        entry.perimeter_edges += 1;
                    }
                }
            }
        }

        let header = vec![
            "segment_id".to_string(),
            "area_px".to_string(),
            "perimeter_px".to_string(),
            "compactness".to_string(),
            "bbox_width_px".to_string(),
            "bbox_height_px".to_string(),
            "elongation".to_string(),
        ];

        let mut ids: Vec<i64> = stats.keys().copied().collect();
        ids.sort_unstable();

        let mut rows_out = Vec::with_capacity(ids.len());
        for seg_id in ids {
            let s = stats.get(&seg_id).expect("shape stats must exist");
            let area = s.area_px as f64;
            let perimeter = s.perimeter_edges as f64;
            let compactness = if perimeter > 0.0 {
                (4.0 * std::f64::consts::PI * area) / (perimeter * perimeter)
            } else {
                0.0
            };
            let width = (s.max_col - s.min_col + 1).max(1) as f64;
            let height = (s.max_row - s.min_row + 1).max(1) as f64;
            let elongation = if width >= height {
                width / height.max(1.0)
            } else {
                height / width.max(1.0)
            };

            rows_out.push(vec![
                seg_id.to_string(),
                s.area_px.to_string(),
                s.perimeter_edges.to_string(),
                compactness.to_string(),
                width.to_string(),
                height.to_string(),
                elongation.to_string(),
            ]);
        }

        write_csv(&output_path, &header, &rows_out)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), serde_json::json!(output_path));
        Ok(ToolRunResult { outputs })
    }
}

pub struct ObjectFeaturesTextureGlcmBasicTool;

impl Tool for ObjectFeaturesTextureGlcmBasicTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "object_features_texture_glcm_basic",
            display_name: "Object Features Texture GLCM Basic",
            summary: "Computes per-segment basic GLCM texture metrics (contrast, homogeneity, energy, entropy).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "segments", description: "Input segment-label raster.", required: true },
                ToolParamSpec { name: "input", description: "Single-band intensity raster for texture analysis.", required: true },
                ToolParamSpec { name: "levels", description: "Quantization levels for GLCM (default 16).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("segments".to_string(), serde_json::json!("segments.tif"));
                d.insert("input".to_string(), serde_json::json!("gray.tif"));
                d.insert("levels".to_string(), serde_json::json!(16));
                d
            },
            examples: vec![ToolExample {
                name: "texture_features".to_string(),
                description: "Extract object-level basic GLCM metrics.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("segments".to_string(), serde_json::json!("segments_clean.tif"));
                    a.insert("input".to_string(), serde_json::json!("nir.tif"));
                    a.insert("levels".to_string(), serde_json::json!(16));
                    a.insert("output".to_string(), serde_json::json!("object_features_texture.csv"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "features".to_string(),
                "texture".to_string(),
                "glcm".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_path_arg(args, "segments")?;
        let _ = parse_required_path_arg(args, "input")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let segments_path = parse_required_path_arg(args, "segments")?;
        let input_path = parse_required_path_arg(args, "input")?;
        let levels = parse_usize_arg(args, "levels", 16).clamp(4, 64);
        let output_path = output_csv_path_or_default(
            &segments_path,
            "object_features_texture",
            parse_optional_path_arg(args, "output"),
        );

        let segments = Raster::read(&segments_path)
            .map_err(|e| ToolError::Execution(format!("failed reading segments raster: {e}")))?;
        let gray = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading texture raster: {e}")))?;

        if segments.rows != gray.rows || segments.cols != gray.cols {
            return Err(ToolError::Validation(
                "segments and input rasters must share dimensions".to_string(),
            ));
        }

        let rows = segments.rows as isize;
        let cols = segments.cols as isize;

        // Determine quantization bounds from valid input cells.
        let mut min_v = f64::INFINITY;
        let mut max_v = -f64::INFINITY;
        for row in 0..rows {
            for col in 0..cols {
                let z = gray.get(0, row, col);
                if gray.is_nodata(z) {
                    continue;
                }
                min_v = min_v.min(z);
                max_v = max_v.max(z);
            }
        }
        if !min_v.is_finite() || !max_v.is_finite() {
            return Err(ToolError::Validation(
                "input texture raster contains no valid cells".to_string(),
            ));
        }
        let range = (max_v - min_v).max(1e-12);

        let quantize = |v: f64| -> usize {
            let q = (((v - min_v) / range) * (levels as f64 - 1.0)).round();
            q.clamp(0.0, levels as f64 - 1.0) as usize
        };

        let mut glcm_by_segment: HashMap<i64, Vec<f64>> = HashMap::new();
        let mut pair_count: HashMap<i64, f64> = HashMap::new();

        let offsets = [(0isize, 1isize), (1isize, 0isize)];
        for row in 0..rows {
            for col in 0..cols {
                let s0 = segments.get(0, row, col);
                if segments.is_nodata(s0) || s0 <= 0.0 {
                    continue;
                }
                let seg_id = s0.round() as i64;
                let z0 = gray.get(0, row, col);
                if gray.is_nodata(z0) {
                    continue;
                }
                let q0 = quantize(z0);

                for (dr, dc) in offsets {
                    let nr = row + dr;
                    let nc = col + dc;
                    if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                        continue;
                    }
                    let s1 = segments.get(0, nr, nc);
                    if segments.is_nodata(s1) || s1.round() as i64 != seg_id {
                        continue;
                    }
                    let z1 = gray.get(0, nr, nc);
                    if gray.is_nodata(z1) {
                        continue;
                    }
                    let q1 = quantize(z1);

                    let m = glcm_by_segment
                        .entry(seg_id)
                        .or_insert_with(|| vec![0.0; levels * levels]);
                    m[q0 * levels + q1] += 1.0;
                    m[q1 * levels + q0] += 1.0;
                    *pair_count.entry(seg_id).or_insert(0.0) += 2.0;
                }
            }
        }

        let header = vec![
            "segment_id".to_string(),
            "pair_count".to_string(),
            "glcm_contrast".to_string(),
            "glcm_homogeneity".to_string(),
            "glcm_energy".to_string(),
            "glcm_entropy".to_string(),
        ];

        let mut ids: Vec<i64> = glcm_by_segment.keys().copied().collect();
        ids.sort_unstable();

        let mut rows_out = Vec::with_capacity(ids.len());
        for seg_id in ids {
            let m = glcm_by_segment
                .get(&seg_id)
                .expect("GLCM matrix should exist for segment");
            let total = *pair_count.get(&seg_id).unwrap_or(&0.0);
            if total <= 0.0 {
                continue;
            }

            let mut contrast = 0.0;
            let mut homogeneity = 0.0;
            let mut energy = 0.0;
            let mut entropy = 0.0;

            for i in 0..levels {
                for j in 0..levels {
                    let p = m[i * levels + j] / total;
                    if p <= 0.0 {
                        continue;
                    }
                    let diff = (i as f64 - j as f64).abs();
                    let diff2 = diff * diff;
                    contrast += p * diff2;
                    homogeneity += p / (1.0 + diff2);
                    energy += p * p;
                    entropy -= p * p.log2();
                }
            }

            rows_out.push(vec![
                seg_id.to_string(),
                total.to_string(),
                contrast.to_string(),
                homogeneity.to_string(),
                energy.to_string(),
                entropy.to_string(),
            ]);
        }

        write_csv(&output_path, &header, &rows_out)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), serde_json::json!(output_path));
        Ok(ToolRunResult { outputs })
    }
}

pub struct ClassifyObjectsRandomForestTool;

impl Tool for ClassifyObjectsRandomForestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "classify_objects_random_forest",
            display_name: "Classify Objects Random Forest",
            summary: "Classifies object records from feature CSV using random forest and segment-linked training labels.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "features", description: "Input object-features CSV (must include segment_id and numeric feature columns).", required: true },
                ToolParamSpec { name: "training", description: "Training CSV with segment_id and class label columns.", required: true },
                ToolParamSpec { name: "segment_id_field", description: "Segment ID field name (default segment_id).", required: false },
                ToolParamSpec { name: "class_field", description: "Class label field in training CSV (default class).", required: false },
                ToolParamSpec { name: "n_trees", description: "Number of trees (default 200).", required: false },
                ToolParamSpec { name: "output", description: "Output predictions CSV path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("segment_id_field".to_string(), serde_json::json!("segment_id"));
                d.insert("class_field".to_string(), serde_json::json!("class"));
                d.insert("n_trees".to_string(), serde_json::json!(200));
                d
            },
            examples: vec![ToolExample {
                name: "classify_objects_rf".to_string(),
                description: "Train and apply object-level random forest model.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("features".to_string(), serde_json::json!("object_features_all.csv"));
                    a.insert("training".to_string(), serde_json::json!("training_segments.csv"));
                    a.insert("output".to_string(), serde_json::json!("object_predictions.csv"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "classification".to_string(),
                "random_forest".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_path_arg(args, "features")?;
        let _ = parse_required_path_arg(args, "training")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let features_path = parse_required_path_arg(args, "features")?;
        let training_path = parse_required_path_arg(args, "training")?;
        let seg_field = args
            .get("segment_id_field")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("segment_id");
        let class_field = args
            .get("class_field")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("class");
        let n_trees = parse_usize_arg(args, "n_trees", 200).max(10) as u16;
        let output_path = output_csv_path_or_default(
            &features_path,
            "object_predictions",
            parse_optional_path_arg(args, "output"),
        );

        let (f_headers, f_rows) = parse_simple_csv(&features_path)?;
        let seg_col = find_header_index(&f_headers, seg_field)?;

        let mut feature_cols = Vec::new();
        for (i, h) in f_headers.iter().enumerate() {
            if i != seg_col {
                feature_cols.push((i, h.clone()));
            }
        }
        if feature_cols.is_empty() {
            return Err(ToolError::Validation(
                "features CSV must contain at least one numeric feature column".to_string(),
            ));
        }

        let mut features_by_segment: HashMap<String, Vec<f64>> = HashMap::new();
        for row in &f_rows {
            let seg_id = row[seg_col].clone();
            let mut vals = Vec::with_capacity(feature_cols.len());
            for (idx, name) in &feature_cols {
                let parsed = row[*idx].parse::<f64>().map_err(|_| {
                    ToolError::Validation(format!(
                        "feature column '{name}' contains non-numeric value '{}'",
                        row[*idx]
                    ))
                })?;
                vals.push(parsed);
            }
            features_by_segment.insert(seg_id, vals);
        }

        let (t_headers, t_rows) = parse_simple_csv(&training_path)?;
        let t_seg_col = find_header_index(&t_headers, seg_field)?;
        let t_class_col = find_header_index(&t_headers, class_field)?;

        let mut class_to_id: HashMap<String, i32> = HashMap::new();
        let mut id_to_class: HashMap<i32, String> = HashMap::new();

        let mut x_train = Vec::<Vec<f64>>::new();
        let mut y_train = Vec::<i32>::new();

        for row in &t_rows {
            let seg_id = &row[t_seg_col];
            let class_name = row[t_class_col].clone();
            let Some(features) = features_by_segment.get(seg_id) else {
                continue;
            };
            let class_id = if let Some(cid) = class_to_id.get(&class_name) {
                *cid
            } else {
                let cid = class_to_id.len() as i32;
                class_to_id.insert(class_name.clone(), cid);
                id_to_class.insert(cid, class_name.clone());
                cid
            };
            x_train.push(features.clone());
            y_train.push(class_id);
        }

        if x_train.len() < 2 {
            return Err(ToolError::Validation(
                "insufficient matched training rows; ensure training segment IDs exist in features CSV"
                    .to_string(),
            ));
        }

        let x_matrix = DenseMatrix::from_2d_vec(&x_train)
            .map_err(|e| ToolError::Execution(format!("failed building feature matrix: {e}")))?;

        let rf = RandomForestClassifier::fit(
            &x_matrix,
            &y_train,
            RandomForestClassifierParameters {
                n_trees,
                ..Default::default()
            },
        )
        .map_err(|e| ToolError::Execution(format!("random forest fit failed: {e}")))?;

        let mut segment_ids: Vec<String> = features_by_segment.keys().cloned().collect();
        segment_ids.sort_by(|a, b| {
            let na = a.parse::<f64>();
            let nb = b.parse::<f64>();
            match (na, nb) {
                (Ok(va), Ok(vb)) => va.partial_cmp(&vb).unwrap_or(Ordering::Equal),
                _ => a.cmp(b),
            }
        });

        let mut x_all = Vec::<Vec<f64>>::with_capacity(segment_ids.len());
        for seg_id in &segment_ids {
            x_all.push(
                features_by_segment
                    .get(seg_id)
                    .expect("features must exist for segment")
                    .clone(),
            );
        }

        let x_all_matrix = DenseMatrix::from_2d_vec(&x_all)
            .map_err(|e| ToolError::Execution(format!("failed building inference matrix: {e}")))?;

        let y_pred = rf
            .predict(&x_all_matrix)
            .map_err(|e| ToolError::Execution(format!("random forest predict failed: {e}")))?;

        let header = vec!["segment_id".to_string(), "predicted_class".to_string()];
        let mut rows_out = Vec::with_capacity(segment_ids.len());
        for (seg_id, pred_id) in segment_ids.iter().zip(y_pred.iter()) {
            let class_name = id_to_class
                .get(pred_id)
                .cloned()
                .unwrap_or_else(|| pred_id.to_string());
            rows_out.push(vec![seg_id.clone(), class_name]);
        }

        write_csv(&output_path, &header, &rows_out)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), serde_json::json!(output_path));
        Ok(ToolRunResult { outputs })
    }
}

pub struct EvaluateObjectClassificationAccuracyTool;

impl Tool for EvaluateObjectClassificationAccuracyTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "evaluate_object_classification_accuracy",
            display_name: "Evaluate Object Classification Accuracy",
            summary: "Evaluates object-level classification predictions against reference labels and outputs OA and kappa.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "predictions", description: "Predictions CSV containing segment_id and predicted_class.", required: true },
                ToolParamSpec { name: "reference", description: "Reference CSV containing segment_id and class.", required: true },
                ToolParamSpec { name: "segment_id_field", description: "Segment ID field name (default segment_id).", required: false },
                ToolParamSpec { name: "predicted_field", description: "Predicted class field (default predicted_class).", required: false },
                ToolParamSpec { name: "reference_field", description: "Reference class field (default class).", required: false },
                ToolParamSpec { name: "output", description: "Output JSON report path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("segment_id_field".to_string(), serde_json::json!("segment_id"));
                d.insert("predicted_field".to_string(), serde_json::json!("predicted_class"));
                d.insert("reference_field".to_string(), serde_json::json!("class"));
                d
            },
            examples: vec![ToolExample {
                name: "object_accuracy_report".to_string(),
                description: "Compute OA and kappa for object-class predictions.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("predictions".to_string(), serde_json::json!("object_predictions.csv"));
                    a.insert("reference".to_string(), serde_json::json!("validation_segments.csv"));
                    a.insert("output".to_string(), serde_json::json!("object_accuracy.json"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "classification".to_string(),
                "accuracy".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_path_arg(args, "predictions")?;
        let _ = parse_required_path_arg(args, "reference")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let pred_path = parse_required_path_arg(args, "predictions")?;
        let ref_path = parse_required_path_arg(args, "reference")?;
        let seg_field = args
            .get("segment_id_field")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("segment_id");
        let pred_field = args
            .get("predicted_field")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("predicted_class");
        let ref_field = args
            .get("reference_field")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("class");
        let output_path = output_path_or_default(
            &pred_path,
            "object_accuracy",
            "json",
            parse_optional_path_arg(args, "output"),
        );

        let (p_headers, p_rows) = parse_simple_csv(&pred_path)?;
        let p_seg_col = find_header_index(&p_headers, seg_field)?;
        let p_class_col = find_header_index(&p_headers, pred_field)?;

        let mut pred_map: HashMap<String, String> = HashMap::new();
        for row in &p_rows {
            pred_map.insert(row[p_seg_col].clone(), row[p_class_col].clone());
        }

        let (r_headers, r_rows) = parse_simple_csv(&ref_path)?;
        let r_seg_col = find_header_index(&r_headers, seg_field)?;
        let r_class_col = find_header_index(&r_headers, ref_field)?;

        let mut ref_map: HashMap<String, String> = HashMap::new();
        for row in &r_rows {
            ref_map.insert(row[r_seg_col].clone(), row[r_class_col].clone());
        }

        let mut confusion: HashMap<String, HashMap<String, usize>> = HashMap::new();
        let mut labels: Vec<String> = Vec::new();
        let mut label_set: HashMap<String, ()> = HashMap::new();

        let mut n = 0usize;
        let mut correct = 0usize;

        for (seg_id, ref_class) in &ref_map {
            let Some(pred_class) = pred_map.get(seg_id) else {
                continue;
            };
            n += 1;
            if pred_class == ref_class {
                correct += 1;
            }

            if !label_set.contains_key(ref_class) {
                label_set.insert(ref_class.clone(), ());
                labels.push(ref_class.clone());
            }
            if !label_set.contains_key(pred_class) {
                label_set.insert(pred_class.clone(), ());
                labels.push(pred_class.clone());
            }

            let row = confusion.entry(ref_class.clone()).or_default();
            *row.entry(pred_class.clone()).or_insert(0) += 1;
        }

        if n == 0 {
            return Err(ToolError::Validation(
                "no overlapping segment IDs between predictions and reference".to_string(),
            ));
        }

        labels.sort();
        labels.dedup();

        let mut row_marginal: HashMap<String, f64> = HashMap::new();
        let mut col_marginal: HashMap<String, f64> = HashMap::new();

        for ref_label in &labels {
            let row = confusion.get(ref_label).cloned().unwrap_or_default();
            let row_sum: usize = row.values().sum();
            row_marginal.insert(ref_label.clone(), row_sum as f64);
            for pred_label in &labels {
                let v = row.get(pred_label).copied().unwrap_or(0) as f64;
                *col_marginal.entry(pred_label.clone()).or_insert(0.0) += v;
            }
        }

        let po = correct as f64 / n as f64;
        let mut pe_num = 0.0;
        for label in &labels {
            let r = *row_marginal.get(label).unwrap_or(&0.0);
            let c = *col_marginal.get(label).unwrap_or(&0.0);
            pe_num += r * c;
        }
        let pe = pe_num / (n as f64 * n as f64);
        let kappa = if (1.0 - pe).abs() < 1e-12 {
            0.0
        } else {
            (po - pe) / (1.0 - pe)
        };

        let report = serde_json::json!({
            "n_samples": n,
            "overall_accuracy": po,
            "kappa": kappa,
            "labels": labels,
            "confusion": confusion,
        });

        if let Some(parent) = Path::new(&output_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!(
                    "failed creating output directory '{}': {e}",
                    parent.display()
                ))
            })?;
        }

        let mut file = File::create(&output_path)
            .map_err(|e| ToolError::Execution(format!("failed creating report '{}': {e}", output_path)))?;
        file.write_all(report.to_string().as_bytes())
            .map_err(|e| ToolError::Execution(format!("failed writing report '{}': {e}", output_path)))?;

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), serde_json::json!(output_path));
        outputs.insert("overall_accuracy".to_string(), serde_json::json!(po));
        outputs.insert("kappa".to_string(), serde_json::json!(kappa));
        Ok(ToolRunResult { outputs })
    }
}

pub struct ObiaPipelineBasicTool;

impl Tool for ObiaPipelineBasicTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "obia_pipeline_basic",
            display_name: "OBIA Pipeline Basic",
            summary: "Runs a basic open-core OBIA pipeline: segmentation, small-region merge, spectral/shape feature extraction, and object random-forest classification.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters for segmentation/features.", required: true },
                ToolParamSpec { name: "training", description: "Training CSV with segment_id and class columns.", required: true },
                ToolParamSpec { name: "output_prefix", description: "Output path prefix for generated artifacts.", required: true },
                ToolParamSpec { name: "segment_method", description: "Segmentation method: slic or graph (default slic).", required: false },
                ToolParamSpec { name: "min_size", description: "Minimum segment size for merge cleanup (default 10).", required: false },
                ToolParamSpec { name: "class_field", description: "Training class field name (default class).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults: {
                let mut d = ToolArgs::new();
                d.insert("segment_method".to_string(), serde_json::json!("slic"));
                d.insert("min_size".to_string(), serde_json::json!(10));
                d.insert("class_field".to_string(), serde_json::json!("class"));
                d
            },
            examples: vec![ToolExample {
                name: "obia_basic_pipeline".to_string(),
                description: "Run the baseline open-core OBIA pipeline.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("inputs".to_string(), serde_json::json!(["red.tif", "green.tif", "nir.tif"]));
                    a.insert("training".to_string(), serde_json::json!("training_segments.csv"));
                    a.insert("output_prefix".to_string(), serde_json::json!("results/field01"));
                    a.insert("segment_method".to_string(), serde_json::json!("slic"));
                    a
                },
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "obia".to_string(),
                "workflow".to_string(),
                "open-core".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_list_arg(args, "inputs")?;
        let _ = parse_required_path_arg(args, "training")?;
        let _ = parse_required_path_arg(args, "output_prefix")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        let training = parse_required_path_arg(args, "training")?;
        let output_prefix = parse_required_path_arg(args, "output_prefix")?;
        let segment_method = args
            .get("segment_method")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("slic");
        let min_size = parse_usize_arg(args, "min_size", 10).max(1);
        let class_field = args
            .get("class_field")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("class")
            .to_string();

        let segments_path = format!("{output_prefix}_segments.tif");
        let segments_clean_path = format!("{output_prefix}_segments_clean.tif");
        let spectral_path = format!("{output_prefix}_object_features_spectral.csv");
        let shape_path = format!("{output_prefix}_object_features_shape.csv");
        let features_all_path = format!("{output_prefix}_object_features_all.csv");
        let predictions_path = format!("{output_prefix}_object_predictions.csv");

        // 1) Segmentation
        let mut seg_args = ToolArgs::new();
        seg_args.insert("inputs".to_string(), serde_json::json!(inputs));
        seg_args.insert("output".to_string(), serde_json::json!(segments_path.clone()));
        let seg_res = match segment_method {
            "graph" => SegmentGraphFelzenszwalbTool.run(&seg_args, ctx)?,
            _ => SegmentSlicSuperpixelsTool.run(&seg_args, ctx)?,
        };
        let seg_out = result_path_from_outputs(&seg_res.outputs).unwrap_or(segments_path.clone());

        // 2) Merge undersized regions
        let mut merge_args = ToolArgs::new();
        merge_args.insert("segments".to_string(), serde_json::json!(seg_out.clone()));
        merge_args.insert("min_size".to_string(), serde_json::json!(min_size));
        merge_args.insert("method".to_string(), serde_json::json!("longest"));
        merge_args.insert("output".to_string(), serde_json::json!(segments_clean_path.clone()));
        let merge_res = SegmentsMergeSmallRegionsTool.run(&merge_args, ctx)?;
        let seg_clean = result_path_from_outputs(&merge_res.outputs)
            .unwrap_or(segments_clean_path.clone());

        // 3) Spectral + shape features
        let mut spec_args = ToolArgs::new();
        spec_args.insert("segments".to_string(), serde_json::json!(seg_clean.clone()));
        spec_args.insert(
            "inputs".to_string(),
            args.get("inputs")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        );
        spec_args.insert("output".to_string(), serde_json::json!(spectral_path.clone()));
        let spec_res = ObjectFeaturesSpectralBasicTool.run(&spec_args, ctx)?;
        let spectral_csv = result_path_from_outputs(&spec_res.outputs)
            .unwrap_or(spectral_path.clone());

        let mut shape_args = ToolArgs::new();
        shape_args.insert("segments".to_string(), serde_json::json!(seg_clean.clone()));
        shape_args.insert("output".to_string(), serde_json::json!(shape_path.clone()));
        let shape_res = ObjectFeaturesShapeBasicTool.run(&shape_args, ctx)?;
        let shape_csv = result_path_from_outputs(&shape_res.outputs)
            .unwrap_or(shape_path.clone());

        // 4) Merge feature CSVs on segment_id.
        let (spec_headers, spec_rows) = parse_simple_csv(&spectral_csv)?;
        let (shape_headers, shape_rows) = parse_simple_csv(&shape_csv)?;

        let spec_seg_col = find_header_index(&spec_headers, "segment_id")?;
        let shape_seg_col = find_header_index(&shape_headers, "segment_id")?;

        let mut shape_by_id: HashMap<String, Vec<String>> = HashMap::new();
        for row in &shape_rows {
            let mut payload = Vec::new();
            for (i, v) in row.iter().enumerate() {
                if i != shape_seg_col {
                    payload.push(v.clone());
                }
            }
            shape_by_id.insert(row[shape_seg_col].clone(), payload);
        }

        let mut merged_header = Vec::new();
        merged_header.push("segment_id".to_string());
        for (i, h) in spec_headers.iter().enumerate() {
            if i != spec_seg_col {
                merged_header.push(h.clone());
            }
        }
        for (i, h) in shape_headers.iter().enumerate() {
            if i != shape_seg_col {
                merged_header.push(h.clone());
            }
        }

        let mut merged_rows = Vec::new();
        for row in &spec_rows {
            let seg_id = row[spec_seg_col].clone();
            let Some(shape_payload) = shape_by_id.get(&seg_id) else {
                continue;
            };
            let mut merged = vec![seg_id.clone()];
            for (i, v) in row.iter().enumerate() {
                if i != spec_seg_col {
                    merged.push(v.clone());
                }
            }
            merged.extend(shape_payload.clone());
            merged_rows.push(merged);
        }

        write_csv(&features_all_path, &merged_header, &merged_rows)?;

        // 5) Object RF classification
        let mut cls_args = ToolArgs::new();
        cls_args.insert("features".to_string(), serde_json::json!(features_all_path.clone()));
        cls_args.insert("training".to_string(), serde_json::json!(training));
        cls_args.insert("class_field".to_string(), serde_json::json!(class_field));
        cls_args.insert("segment_id_field".to_string(), serde_json::json!("segment_id"));
        cls_args.insert("output".to_string(), serde_json::json!(predictions_path.clone()));
        let cls_res = ClassifyObjectsRandomForestTool.run(&cls_args, ctx)?;
        let predictions = result_path_from_outputs(&cls_res.outputs)
            .unwrap_or(predictions_path.clone());

        let mut outputs = BTreeMap::new();
        outputs.insert("segments".to_string(), serde_json::json!(seg_out));
        outputs.insert("segments_clean".to_string(), serde_json::json!(seg_clean));
        outputs.insert("features_spectral".to_string(), serde_json::json!(spectral_csv));
        outputs.insert("features_shape".to_string(), serde_json::json!(shape_csv));
        outputs.insert("features_all".to_string(), serde_json::json!(features_all_path));
        outputs.insert("predictions".to_string(), serde_json::json!(predictions));
        Ok(ToolRunResult { outputs })
    }
}
