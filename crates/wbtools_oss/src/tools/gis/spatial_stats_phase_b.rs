//! Phase B kriging and geostatistical inference tools
//!
//! Implements ordinary kriging, local kriging, simple kriging, universal kriging,
//! and spatio-temporal kriging by wrapping the wbspatialstats backend.

use super::*;
use wbspatialstats::kriging::{OrdinaryKriging, LocalOrdinaryKriging, SimpleKriging, UniversalKriging, SpaceTimeKriging};
use wbspatialstats::variogram::{EmpiricalVariogramBuilder, VariogramFitter, VariogramModelFamily};

// Tool marker structs
pub struct OrdinaryKrigingTool;
pub struct LocalOrdinaryKrigingTool;
pub struct SimpleKrigingTool;
pub struct UniversalKrigingTool;
pub struct SpaceTimeKrigingTool;

/// Helper to extract point coordinates and values from a vector layer
fn extract_point_data(
    layer: &wbvector::Layer,
    value_field: &str,
) -> Result<(Vec<(f64, f64)>, Vec<f64>), ToolError> {
    let field_idx = layer
        .schema
        .field_index(value_field)
        .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", value_field)))?;

    let mut coords = Vec::new();
    let mut values = Vec::new();

    for feature in &layer.features {
        if let Some(geom) = &feature.geometry {
            if let wbvector::Geometry::Point(coord) = geom {
                if let Some(val) = feature.attributes.get(field_idx).and_then(|v| v.as_f64()) {
                    if val.is_finite() {
                        coords.push((coord.x, coord.y));
                        values.push(val);
                    }
                }
            }
        }
    }

    if coords.len() < 3 {
        return Err(ToolError::Execution(format!(
            "At least 3 points with valid values required, found {}",
            coords.len()
        )));
    }

    Ok((coords, values))
}

/// Generate a prediction grid covering input point extent
fn generate_prediction_grid(coords: &[(f64, f64)], grid_size: usize) -> (Vec<(f64, f64)>, f64, f64, f64, f64) {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);

    for (x, y) in coords {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }

    let cell_size = ((max_x - min_x).max(max_y - min_y)) / (grid_size as f64);
    let mut grid = Vec::new();

    let mut x = min_x;
    while x <= max_x {
        let mut y = min_y;
        while y <= max_y {
            grid.push((x, y));
            y += cell_size;
        }
        x += cell_size;
    }

    (grid, min_x, max_x, min_y, max_y)
}

// ============================================================================
// ORDINARY KRIGING TOOL
// ============================================================================

impl Tool for OrdinaryKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ordinary_kriging",
            display_name: "Ordinary Kriging",
            summary: "Computes ordinary kriging predictions with kriging variance from point samples.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_points", description: "Vector points with values.", required: true },
                ToolParamSpec { name: "value_field", description: "Attribute field with values to interpolate.", required: true },
                ToolParamSpec { name: "output", description: "Output CSV with kriging statistics.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_points".to_string(), json!("sample_points.shp"));
        defaults.insert("value_field".to_string(), json!("temperature"));
        defaults.insert("output".to_string(), json!("kriging_results.csv"));

        ToolManifest {
            id: "ordinary_kriging".to_string(),
            display_name: "Ordinary Kriging".to_string(),
            summary: "Computes ordinary kriging predictions with kriging variance.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_points".to_string(), description: "Vector points with values.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Attribute field with values to interpolate.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV with results.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "ordinary_kriging_basic".to_string(),
                description: "Performs ordinary kriging interpolation.".to_string(),
                args: defaults,
            }],
            tags: vec!["interpolation".to_string(), "kriging".to_string(), "geostatistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input_points")?;
        args.get("value_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("value_field must be a string".to_string()))?;
        args.get("output")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("output must be a string".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        let value_field = args
            .get("value_field")
            .and_then(|v| v.as_str())
            .unwrap_or("value");
        let output_path = args
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("kriging_results.csv");

        ctx.progress.info(&format!("Extracting coordinates and values from field '{}'", value_field));
        let (coords, values) = extract_point_data(&input, value_field)?;

        ctx.progress.info(&format!("Building empirical variogram with {} points", coords.len()));
        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        ctx.progress.info("Fitting spherical variogram model");
        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info("Creating ordinary kriging engine");
        let kriging = OrdinaryKriging::new(coords.clone(), values.clone(), model)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Generating prediction grid");
        let (grid, _min_x, _max_x, _min_y, _max_y) = generate_prediction_grid(&coords, 50);

        ctx.progress.info(&format!("Computing kriging predictions for {} grid cells", grid.len()));
        let mut csv_output = String::from("x,y,predicted,variance,std_error,ci_lower,ci_upper\n");

        for (idx, (x, y)) in grid.iter().enumerate() {
            match kriging.predict((*x, *y)) {
                Ok(result) => {
                    csv_output.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        x, y, result.prediction, result.variance, 
                        result.std_error, result.ci_lower, result.ci_upper
                    ));
                }
                Err(_) => {
                    csv_output.push_str(&format!("{},{},NaN,NaN,NaN,NaN,NaN\n", x, y));
                }
            }
            if idx % 100 == 0 {
                ctx.progress.progress(idx as f64 / grid.len() as f64);
            }
        }

        ctx.progress.info(&format!("Writing results to {}", output_path));
        std::fs::write(output_path, csv_output)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// LOCAL ORDINARY KRIGING TOOL
// ============================================================================

impl Tool for LocalOrdinaryKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "local_ordinary_kriging",
            display_name: "Local Ordinary Kriging",
            summary: "Computes local ordinary kriging using k-nearest neighbors.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_points", description: "Vector points with values.", required: true },
                ToolParamSpec { name: "value_field", description: "Attribute field with values to interpolate.", required: true },
                ToolParamSpec { name: "k", description: "Number of nearest neighbors (default: 16).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV with kriging results.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("k".to_string(), json!(16));
        defaults.insert("input_points".to_string(), json!("sample_points.shp"));
        defaults.insert("value_field".to_string(), json!("temperature"));
        defaults.insert("output".to_string(), json!("local_kriging_results.csv"));

        ToolManifest {
            id: "local_ordinary_kriging".to_string(),
            display_name: "Local Ordinary Kriging".to_string(),
            summary: "Computes local ordinary kriging using k-nearest neighbors.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_points".to_string(), description: "Vector points with values.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Attribute field with values to interpolate.".to_string(), required: true },
                ToolParamDescriptor { name: "k".to_string(), description: "Number of nearest neighbors.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV with results.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "local_kriging_basic".to_string(),
                description: "Performs local ordinary kriging with 16 nearest neighbors.".to_string(),
                args: defaults,
            }],
            tags: vec!["interpolation".to_string(), "kriging".to_string(), "local".to_string(), "geostatistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input_points")?;
        args.get("value_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("value_field must be a string".to_string()))?;
        args.get("output")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("output must be a string".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        let value_field = args
            .get("value_field")
            .and_then(|v| v.as_str())
            .unwrap_or("value");
        let k = args
            .get("k")
            .and_then(|v| v.as_i64())
            .unwrap_or(16) as usize;
        let output_path = args
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("local_kriging_results.csv");

        ctx.progress.info(&format!("Extracting coordinates and values from field '{}'", value_field));
        let (coords, values) = extract_point_data(&input, value_field)?;

        ctx.progress.info(&format!("Building empirical variogram with {} points", coords.len()));
        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        ctx.progress.info("Fitting spherical variogram model");
        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info(&format!("Creating local kriging engine with k={}", k));
        let kriging = LocalOrdinaryKriging::new(coords.clone(), values.clone(), model, k)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Generating prediction grid");
        let (grid, _min_x, _max_x, _min_y, _max_y) = generate_prediction_grid(&coords, 50);

        ctx.progress.info(&format!("Computing local kriging predictions for {} grid cells", grid.len()));
        let mut csv_output = String::from("x,y,predicted,variance,std_error,ci_lower,ci_upper\n");

        for (idx, (x, y)) in grid.iter().enumerate() {
            match kriging.predict((*x, *y)) {
                Ok(result) => {
                    csv_output.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        x, y, result.prediction, result.variance,
                        result.std_error, result.ci_lower, result.ci_upper
                    ));
                }
                Err(_) => {
                    csv_output.push_str(&format!("{},{},NaN,NaN,NaN,NaN,NaN\n", x, y));
                }
            }
            if idx % 100 == 0 {
                ctx.progress.progress(idx as f64 / grid.len() as f64);
            }
        }

        ctx.progress.info(&format!("Writing results to {}", output_path));
        std::fs::write(output_path, csv_output)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// SIMPLE KRIGING TOOL
// ============================================================================

impl Tool for SimpleKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "simple_kriging",
            display_name: "Simple Kriging",
            summary: "Computes simple kriging with known global mean.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_points", description: "Vector points with values.", required: true },
                ToolParamSpec { name: "value_field", description: "Attribute field with values to interpolate.", required: true },
                ToolParamSpec { name: "global_mean", description: "Known global mean value (default: auto).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV with kriging results.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_points".to_string(), json!("sample_points.shp"));
        defaults.insert("value_field".to_string(), json!("temperature"));
        defaults.insert("output".to_string(), json!("simple_kriging_results.csv"));

        ToolManifest {
            id: "simple_kriging".to_string(),
            display_name: "Simple Kriging".to_string(),
            summary: "Computes simple kriging with known global mean.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_points".to_string(), description: "Vector points with values.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Attribute field with values to interpolate.".to_string(), required: true },
                ToolParamDescriptor { name: "global_mean".to_string(), description: "Known global mean (default: auto).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV with results.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "simple_kriging_basic".to_string(),
                description: "Performs simple kriging with automatic mean estimation.".to_string(),
                args: defaults,
            }],
            tags: vec!["interpolation".to_string(), "kriging".to_string(), "simple".to_string(), "geostatistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input_points")?;
        args.get("value_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("value_field must be a string".to_string()))?;
        args.get("output")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("output must be a string".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        let value_field = args
            .get("value_field")
            .and_then(|v| v.as_str())
            .unwrap_or("value");
        let global_mean: f64 = args
            .get("global_mean")
            .and_then(|v| v.as_f64())
            .unwrap_or_else(|| {
                // Default: compute mean from data
                let (_, values) = extract_point_data(&input, value_field).unwrap_or((Vec::new(), vec![0.0]));
                values.iter().sum::<f64>() / values.len().max(1) as f64
            });
        let output_path = args
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("simple_kriging_results.csv");

        ctx.progress.info(&format!("Extracting coordinates and values from field '{}'", value_field));
        let (coords, values) = extract_point_data(&input, value_field)?;
        let actual_mean = global_mean;

        ctx.progress.info(&format!("Building empirical variogram from residuals (global mean = {:.4})", actual_mean));
        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        ctx.progress.info("Fitting spherical variogram model");
        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info("Creating simple kriging engine");
        let kriging = SimpleKriging::new(coords.clone(), values.clone(), model, actual_mean)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Generating prediction grid");
        let (grid, _min_x, _max_x, _min_y, _max_y) = generate_prediction_grid(&coords, 50);

        ctx.progress.info(&format!("Computing simple kriging predictions for {} grid cells", grid.len()));
        let mut csv_output = String::from("x,y,predicted,variance,std_error,ci_lower,ci_upper\n");

        for (idx, (x, y)) in grid.iter().enumerate() {
            match kriging.predict(*x, *y) {
                Ok(result) => {
                    csv_output.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        x, y, result.prediction, result.variance,
                        result.std_error, result.ci_lower, result.ci_upper
                    ));
                }
                Err(_) => {
                    csv_output.push_str(&format!("{},{},NaN,NaN,NaN,NaN,NaN\n", x, y));
                }
            }
            if idx % 100 == 0 {
                ctx.progress.progress(idx as f64 / grid.len() as f64);
            }
        }

        ctx.progress.info(&format!("Writing results to {}", output_path));
        std::fs::write(output_path, csv_output)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// UNIVERSAL KRIGING TOOL
// ============================================================================

impl Tool for UniversalKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "universal_kriging",
            display_name: "Universal Kriging",
            summary: "Computes universal kriging with polynomial trend.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_points", description: "Vector points with values.", required: true },
                ToolParamSpec { name: "value_field", description: "Attribute field with values to interpolate.", required: true },
                ToolParamSpec { name: "trend_order", description: "Polynomial trend order (0=mean, 1=linear, 2=quadratic).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV with kriging results.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("trend_order".to_string(), json!(1));
        defaults.insert("input_points".to_string(), json!("sample_points.shp"));
        defaults.insert("value_field".to_string(), json!("temperature"));
        defaults.insert("output".to_string(), json!("universal_kriging_results.csv"));

        ToolManifest {
            id: "universal_kriging".to_string(),
            display_name: "Universal Kriging".to_string(),
            summary: "Computes universal kriging with polynomial trend.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_points".to_string(), description: "Vector points with values.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Attribute field with values to interpolate.".to_string(), required: true },
                ToolParamDescriptor { name: "trend_order".to_string(), description: "Polynomial trend order (0=mean, 1=linear, 2=quadratic).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV with results.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "universal_kriging_linear".to_string(),
                description: "Performs universal kriging with linear trend.".to_string(),
                args: defaults,
            }],
            tags: vec!["interpolation".to_string(), "kriging".to_string(), "trend".to_string(), "geostatistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input_points")?;
        args.get("value_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("value_field must be a string".to_string()))?;
        args.get("output")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("output must be a string".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        let value_field = args
            .get("value_field")
            .and_then(|v| v.as_str())
            .unwrap_or("value");
        let trend_order = args
            .get("trend_order")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as usize;
        let output_path = args
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("universal_kriging_results.csv");

        if trend_order > 2 {
            return Err(ToolError::Validation(
                "trend_order must be 0 (mean), 1 (linear), or 2 (quadratic)".to_string(),
            ));
        }

        ctx.progress.info(&format!("Extracting coordinates and values from field '{}'", value_field));
        let (coords, values) = extract_point_data(&input, value_field)?;

        ctx.progress.info(&format!("Building empirical variogram with {} points", coords.len()));
        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        ctx.progress.info("Fitting spherical variogram model");
        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info(&format!("Creating universal kriging engine with trend order {}", trend_order));
        let kriging = UniversalKriging::new(coords.clone(), values.clone(), model, trend_order)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Generating prediction grid");
        let (grid, _min_x, _max_x, _min_y, _max_y) = generate_prediction_grid(&coords, 50);

        ctx.progress.info(&format!("Computing universal kriging predictions for {} grid cells", grid.len()));
        let mut csv_output = String::from("x,y,predicted,variance,std_error,ci_lower,ci_upper\n");

        for (idx, (x, y)) in grid.iter().enumerate() {
            match kriging.predict(*x, *y) {
                Ok(result) => {
                    csv_output.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        x, y, result.prediction, result.variance,
                        result.std_error, result.ci_lower, result.ci_upper
                    ));
                }
                Err(_) => {
                    csv_output.push_str(&format!("{},{},NaN,NaN,NaN,NaN,NaN\n", x, y));
                }
            }
            if idx % 100 == 0 {
                ctx.progress.progress(idx as f64 / grid.len() as f64);
            }
        }

        ctx.progress.info(&format!("Writing results to {}", output_path));
        std::fs::write(output_path, csv_output)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// SPATIO-TEMPORAL KRIGING TOOL
// ============================================================================

impl Tool for SpaceTimeKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spacetime_kriging",
            display_name: "Spatio-Temporal Kriging",
            summary: "Computes spatio-temporal kriging for space-time prediction.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_points", description: "Vector points with values and time.", required: true },
                ToolParamSpec { name: "value_field", description: "Attribute field with values to interpolate.", required: true },
                ToolParamSpec { name: "time_field", description: "Attribute field with time values.", required: true },
                ToolParamSpec { name: "output", description: "Output CSV with kriging results.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_points".to_string(), json!("sample_points.shp"));
        defaults.insert("value_field".to_string(), json!("temperature"));
        defaults.insert("time_field".to_string(), json!("year"));
        defaults.insert("output".to_string(), json!("spacetime_kriging_results.csv"));

        ToolManifest {
            id: "spacetime_kriging".to_string(),
            display_name: "Spatio-Temporal Kriging".to_string(),
            summary: "Computes spatio-temporal kriging for space-time prediction.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_points".to_string(), description: "Vector points with values and time.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Attribute field with values to interpolate.".to_string(), required: true },
                ToolParamDescriptor { name: "time_field".to_string(), description: "Attribute field with time values.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV with results.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "spacetime_kriging_basic".to_string(),
                description: "Performs spatio-temporal kriging on space-time point data.".to_string(),
                args: defaults,
            }],
            tags: vec!["interpolation".to_string(), "kriging".to_string(), "temporal".to_string(), "geostatistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input_points")?;
        args.get("value_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("value_field must be a string".to_string()))?;
        args.get("time_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("time_field must be a string".to_string()))?;
        args.get("output")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("output must be a string".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        let value_field = args
            .get("value_field")
            .and_then(|v| v.as_str())
            .unwrap_or("value");
        let time_field = args
            .get("time_field")
            .and_then(|v| v.as_str())
            .unwrap_or("time");
        let output_path = args
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("spacetime_kriging_results.csv");

        ctx.progress.info(&format!(
            "Extracting coordinates, times, and values from fields '{}' and '{}'",
            value_field, time_field
        ));

        let value_idx = input
            .schema
            .field_index(value_field)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", value_field)))?;
        let time_idx = input
            .schema
            .field_index(time_field)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", time_field)))?;

        let mut coords_spatial = Vec::new();
        let mut coords_temporal = Vec::new();
        let mut values = Vec::new();

        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    if let (Some(val), Some(time)) = (
                        feature.attributes.get(value_idx).and_then(|v| v.as_f64()),
                        feature.attributes.get(time_idx).and_then(|v| v.as_f64()),
                    ) {
                        if val.is_finite() && time.is_finite() {
                            coords_spatial.push((coord.x, coord.y));
                            coords_temporal.push(time);
                            values.push(val);
                        }
                    }
                }
            }
        }

        if coords_spatial.len() < 4 {
            return Err(ToolError::Execution(format!(
                "At least 4 points with valid values and times required, found {}",
                coords_spatial.len()
            )));
        }

        ctx.progress.info(&format!("Building spatial variogram with {} points", coords_spatial.len()));
        let builder = EmpiricalVariogramBuilder::default();
        let spatial_vario = builder
            .build(&coords_spatial, &values)
            .map_err(|e| ToolError::Execution(format!("Spatial variogram estimation failed: {}", e)))?;

        ctx.progress.info("Fitting spatial variogram model");
        let spatial_model = VariogramFitter::fit(&spatial_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Spatial variogram fitting failed: {}", e)))?;

        ctx.progress.info("Building temporal variogram");
        let builder_temporal = EmpiricalVariogramBuilder::default();
        let temporal_vario = builder_temporal
            .build(&coords_temporal.iter().map(|&t| (t, 0.0)).collect::<Vec<_>>(), &values)
            .map_err(|e| ToolError::Execution(format!("Temporal variogram estimation failed: {}", e)))?;

        ctx.progress.info("Fitting temporal variogram model");
        let temporal_model = VariogramFitter::fit(&temporal_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Temporal variogram fitting failed: {}", e)))?;

        ctx.progress.info("Creating space-time kriging engine");
        let kriging = SpaceTimeKriging::new(coords_spatial.clone(), coords_temporal.clone(), values.clone(), spatial_model, temporal_model)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Generating space-time prediction grid");
        let (grid_2d, _min_x, _max_x, _min_y, _max_y) = generate_prediction_grid(&coords_spatial, 50);

        // Get time range from data
        let min_time = coords_temporal.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time = coords_temporal.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let time_step = (max_time - min_time) / 4.0_f64.max(1.0);

        let mut grid_3d = Vec::new();
        let mut t = min_time;
        while t <= max_time {
            for (x, y) in &grid_2d {
                grid_3d.push((*x, *y, t));
            }
            t += time_step;
        }

        ctx.progress.info(&format!("Computing space-time kriging predictions for {} grid cells", grid_3d.len()));
        let mut csv_output = String::from("x,y,time,predicted,variance,std_error,ci_lower,ci_upper\n");

        for (idx, (x, y, t)) in grid_3d.iter().enumerate() {
            match kriging.predict(*x, *y, *t) {
                Ok(result) => {
                    csv_output.push_str(&format!(
                        "{},{},{},{},{},{},{},{}\n",
                        x, y, t, result.prediction, result.variance,
                        result.std_error, result.ci_lower, result.ci_upper
                    ));
                }
                Err(_) => {
                    csv_output.push_str(&format!("{},{},{},NaN,NaN,NaN,NaN,NaN\n", x, y, t));
                }
            }
            if idx % 100 == 0 {
                ctx.progress.progress(idx as f64 / grid_3d.len() as f64);
            }
        }

        ctx.progress.info(&format!("Writing results to {}", output_path));
        std::fs::write(output_path, csv_output)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}
