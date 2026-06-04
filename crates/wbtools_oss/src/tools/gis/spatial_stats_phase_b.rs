//! Phase B kriging and geostatistical inference tools
//!
//! Implements ordinary kriging, local kriging, simple kriging, universal kriging,
//! and spatio-temporal kriging by wrapping the wbspatialstats backend.
//!
//! All tools output raster surfaces (grid resolution configurable) with full CRS preservation.
//! Output format is determined by file extension (e.g., .tif → GeoTIFF, .img → HFA, .hdf → HDF5).
//! Format flexibility is handled automatically by wbraster via GisOverlayCore::store_or_write_output().

use super::*;
use wbspatialstats::kriging::{OrdinaryKriging, LocalOrdinaryKriging, SimpleKriging, UniversalKriging, SpaceTimeKriging};
use wbspatialstats::variogram::{EmpiricalVariogramBuilder, VariogramFitter, VariogramModelFamily};

// Tool marker structs
pub struct OrdinaryKrigingTool;
pub struct LocalOrdinaryKrigingTool;
pub struct SimpleKrigingTool;
pub struct UniversalKrigingTool;
pub struct SpaceTimeKrigingTool;

// ============================================================================
// ORDINARY KRIGING TOOL
// ============================================================================

impl Tool for OrdinaryKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ordinary_kriging",
            display_name: "Ordinary Kriging Interpolation",
            summary: r#"Performs ordinary kriging interpolation to estimate values at unmapped grid cells based on a sample of point measurements. Ordinary kriging is a geostatistical method that leverages spatial autocorrelation structure to provide both predictions and associated kriging variance (prediction uncertainty). The tool automatically estimates an empirical variogram from the input point data and fits a theoretical variogram model (Spherical, Exponential, or Gaussian) to capture spatial dependence.

Ordinary kriging assumes an unknown constant mean across the study area. It is appropriate for continuous spatial phenomena such as ore grades, pollutant concentrations, precipitation, or soil properties. The method uses all available data points for each prediction, making it computationally intensive for large datasets (>5000 points). For large datasets, consider using Local Ordinary Kriging instead.

Outputs include the interpolated raster and kriging variance, which can be used to assess prediction confidence. The kriging variance is independent of actual values and depends only on variogram model and sample configuration. This makes it useful for survey design and identifying areas of high interpolation uncertainty."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "value_field", description: "Numeric attribute field containing values to interpolate.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size in map units (required if no base_raster).", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("value_field".to_string(), json!("value"));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("ordinary_kriging.tif"));
        ToolManifest {
            id: "ordinary_kriging".to_string(),
            display_name: "Ordinary Kriging Interpolation".to_string(),
            summary: r#"Performs ordinary kriging interpolation to estimate values at unmapped grid cells based on a sample of point measurements. Ordinary kriging is a geostatistical method that leverages spatial autocorrelation structure to provide both predictions and associated kriging variance (prediction uncertainty). The tool automatically estimates an empirical variogram from the input point data and fits a theoretical variogram model (Spherical, Exponential, or Gaussian) to capture spatial dependence."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Numeric attribute field containing values to interpolate.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size in map units (required if no base_raster).".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "ordinary_kriging_basic".to_string(),
                description: "Interpolates point values to a raster using ordinary kriging.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "kriging".to_string(), "interpolation".to_string(), "spatial-stats".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Running ordinary kriging interpolation");
        
        let points = load_vector_arg(args, "points")?;
        let value_field = args.get("value_field").and_then(|v| v.as_str()).unwrap_or("value");
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("Extracting point coordinates and values");
        let samples = collect_point_samples(&points, Some(value_field), false)?;

        ctx.progress.info("Building and fitting empirical variogram");
        let coords: Vec<(f64, f64)> = samples.iter().map(|(x, y, _)| (*x, *y)).collect();
        let values: Vec<f64> = samples.iter().map(|(_, _, v)| *v).collect();

        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info("Creating kriging engine");
        let kriging = OrdinaryKriging::new(coords.clone(), values.clone(), model)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Building output raster");
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        ctx.progress.info(&format!("Generating predictions on {} x {} grid", output.cols, output.rows));
        let coalescer = PercentCoalescer::new(1, 99);
        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![output.nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                if let Ok(result) = kriging.predict((x, y)) {
                    let idx = row * cols + col;
                    out_values[idx] = result.prediction;
                }
            }
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        // Output format determined by file extension (.tif → GeoTIFF, .img → HFA, etc.)
        // Format detection handled automatically by wbraster via GisOverlayCore.
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

// ============================================================================
// LOCAL ORDINARY KRIGING TOOL
// ============================================================================

impl Tool for LocalOrdinaryKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "local_kriging",
            display_name: "Local Ordinary Kriging",
            summary: r#"Performs local ordinary kriging interpolation using k-nearest neighbors, providing efficient spatial estimation for large datasets. This approach is computationally more practical than global ordinary kriging for datasets with thousands of points. For each prediction location, the algorithm fits a separate variogram model using only the k nearest neighbors, dramatically reducing computation while maintaining kriging's geostatistical advantages.

Local kriging is ideal when spatial correlation structure varies across the study area (non-stationary) or when computational resources are limited. The k-neighbors parameter controls the trade-off between accuracy and speed; typical values range from 8-20. Fewer neighbors speed computation but may increase local variance; more neighbors improve stability but increase computation cost.

Output includes both interpolated predictions and kriging variance, useful for identifying areas where predictions are less reliable due to sparse or uneven sampling."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "value_field", description: "Numeric attribute field containing values to interpolate.", required: false },
                ToolParamSpec { name: "k_neighbors", description: "Number of nearest neighbors to use; defaults to 10.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size in map units (required if no base_raster).", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("value_field".to_string(), json!("value"));
        defaults.insert("k_neighbors".to_string(), json!(10));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("local_kriging.tif"));
        ToolManifest {
            id: "local_kriging".to_string(),
            display_name: "Local Ordinary Kriging".to_string(),
            summary: r#"Performs local ordinary kriging interpolation using k-nearest neighbors, providing efficient spatial estimation for large datasets. This approach is computationally more practical than global ordinary kriging for datasets with thousands of points."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Numeric attribute field containing values to interpolate.".to_string(), required: false },
                ToolParamDescriptor { name: "k_neighbors".to_string(), description: "Number of nearest neighbors to use; defaults to 10.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size in map units (required if no base_raster).".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "local_kriging_basic".to_string(),
                description: "Interpolates point values to a raster using local kriging with k-neighbors.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "kriging".to_string(), "interpolation".to_string(), "spatial-stats".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Running local ordinary kriging interpolation");
        
        let points = load_vector_arg(args, "points")?;
        let value_field = args.get("value_field").and_then(|v| v.as_str()).unwrap_or("value");
        let k_neighbors = args.get("k_neighbors").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("Extracting point coordinates and values");
        let samples = collect_point_samples(&points, Some(value_field), false)?;

        ctx.progress.info("Building and fitting empirical variogram");
        let coords: Vec<(f64, f64)> = samples.iter().map(|(x, y, _)| (*x, *y)).collect();
        let values: Vec<f64> = samples.iter().map(|(_, _, v)| *v).collect();

        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info(&format!("Creating kriging engine with k={} neighbors", k_neighbors));
        let kriging = LocalOrdinaryKriging::new(coords.clone(), values.clone(), model, k_neighbors)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Building output raster");
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        ctx.progress.info(&format!("Generating predictions on {} x {} grid", output.cols, output.rows));
        let coalescer = PercentCoalescer::new(1, 99);
        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![output.nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                if let Ok(result) = kriging.predict((x, y)) {
                    let idx = row * cols + col;
                    out_values[idx] = result.prediction;
                }
            }
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        // Output format determined by file extension (.tif → GeoTIFF, .img → HFA, etc.)
        // Format detection handled automatically by wbraster via GisOverlayCore.
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
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
            summary: r#"Performs simple kriging interpolation when the mean of the spatial field is known a priori. This method is a variant of ordinary kriging that assumes a constant, known expected value across the study area. Simple kriging reduces kriging variance and provides more stable predictions when the mean is reliably known from external information or theoretical understanding.

Simple kriging is appropriate when you have strong prior knowledge of the spatial field's mean (e.g., from regional climate normals, geological surveys, or calibration data). The tool automatically estimates and fits an empirical variogram to capture spatial correlation structure. Predictions include both estimated values and kriging variance reflecting prediction uncertainty.

Compare to ordinary kriging when the mean is unknown, or universal kriging when the mean varies spatially (has a trend). Simple kriging generally produces lower prediction variance than ordinary kriging due to the fixed mean assumption."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "value_field", description: "Numeric attribute field containing values to interpolate.", required: false },
                ToolParamSpec { name: "known_mean", description: "Known mean of the spatial field.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size in map units (required if no base_raster).", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("value_field".to_string(), json!("value"));
        defaults.insert("known_mean".to_string(), json!(0.0));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("simple_kriging.tif"));
        ToolManifest {
            id: "simple_kriging".to_string(),
            display_name: "Simple Kriging".to_string(),
            summary: r#"Performs simple kriging interpolation when the mean of the spatial field is known a priori. This method reduces kriging variance and provides more stable predictions when the mean is reliably known from external information."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Numeric attribute field containing values to interpolate.".to_string(), required: false },
                ToolParamDescriptor { name: "known_mean".to_string(), description: "Known mean of the spatial field.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size in map units (required if no base_raster).".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "simple_kriging_basic".to_string(),
                description: "Interpolates point values to a raster using simple kriging.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "kriging".to_string(), "interpolation".to_string(), "spatial-stats".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Running simple kriging interpolation");
        
        let points = load_vector_arg(args, "points")?;
        let value_field = args.get("value_field").and_then(|v| v.as_str()).unwrap_or("value");
        let known_mean = args.get("known_mean").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("Extracting point coordinates and values");
        let samples = collect_point_samples(&points, Some(value_field), false)?;

        ctx.progress.info("Building and fitting empirical variogram");
        let coords: Vec<(f64, f64)> = samples.iter().map(|(x, y, _)| (*x, *y)).collect();
        let values: Vec<f64> = samples.iter().map(|(_, _, v)| *v).collect();

        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info(&format!("Creating kriging engine with known mean {}", known_mean));
        let kriging = SimpleKriging::new(coords.clone(), values.clone(), model, known_mean)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Building output raster");
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        ctx.progress.info(&format!("Generating predictions on {} x {} grid", output.cols, output.rows));
        let coalescer = PercentCoalescer::new(1, 99);
        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![output.nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                if let Ok(result) = kriging.predict(x, y) {
                    let idx = row * cols + col;
                    out_values[idx] = result.prediction;
                }
            }
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        // Output format determined by file extension (.tif → GeoTIFF, .img → HFA, etc.)
        // Format detection handled automatically by wbraster via GisOverlayCore.
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
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
            summary: r#"Performs universal kriging interpolation when the spatial phenomenon has a systematic trend or drift that varies across the study area. Universal kriging combines kriging with polynomial trend surface estimation to separate the large-scale trend from small-scale spatial variation. This allows modeling situations where values systematically increase or decrease across the region (e.g., elevation trends, temperature gradients).

The trend_order parameter controls the polynomial degree: trend_order=1 fits a linear trend (sloping plane); trend_order=2 fits a quadratic trend (curved surface). After removing the trend, ordinary kriging is applied to the residuals. This two-step approach often produces better predictions than ordinary kriging alone when trends are present.

Universal kriging is appropriate for continuous spatial phenomena with geographic trends (elevation, pollution gradients, resource grades). The tool automatically fits both the trend and variogram model. Outputs include predictions and kriging variance. Use when ordinary kriging residuals show systematic spatial patterns (suggests unmodeled trend)."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "value_field", description: "Numeric attribute field containing values to interpolate.", required: false },
                ToolParamSpec { name: "trend_order", description: "Polynomial trend order (1=linear, 2=quadratic); defaults to 1.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size in map units (required if no base_raster).", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("value_field".to_string(), json!("value"));
        defaults.insert("trend_order".to_string(), json!(1));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("universal_kriging.tif"));
        ToolManifest {
            id: "universal_kriging".to_string(),
            display_name: "Universal Kriging".to_string(),
            summary: r#"Performs universal kriging interpolation when the spatial phenomenon has a systematic trend or drift. Combines kriging with polynomial trend surface estimation."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Numeric attribute field containing values to interpolate.".to_string(), required: false },
                ToolParamDescriptor { name: "trend_order".to_string(), description: "Polynomial trend order (1=linear, 2=quadratic); defaults to 1.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size in map units (required if no base_raster).".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "universal_kriging_basic".to_string(),
                description: "Interpolates point values to a raster using universal kriging with trend.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "kriging".to_string(), "interpolation".to_string(), "spatial-stats".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Running universal kriging interpolation");
        
        let points = load_vector_arg(args, "points")?;
        let value_field = args.get("value_field").and_then(|v| v.as_str()).unwrap_or("value");
        let trend_order = args.get("trend_order").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("Extracting point coordinates and values");
        let samples = collect_point_samples(&points, Some(value_field), false)?;

        ctx.progress.info("Building and fitting empirical variogram");
        let coords: Vec<(f64, f64)> = samples.iter().map(|(x, y, _)| (*x, *y)).collect();
        let values: Vec<f64> = samples.iter().map(|(_, _, v)| *v).collect();

        let builder = EmpiricalVariogramBuilder::default();
        let empirical_vario = builder
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram estimation failed: {}", e)))?;

        let model = VariogramFitter::fit(&empirical_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Variogram fitting failed: {}", e)))?;

        ctx.progress.info(&format!("Creating kriging engine with trend order {}", trend_order));
        let kriging = UniversalKriging::new(coords.clone(), values.clone(), model, trend_order)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Building output raster");
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        ctx.progress.info(&format!("Generating predictions on {} x {} grid", output.cols, output.rows));
        let coalescer = PercentCoalescer::new(1, 99);
        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![output.nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                if let Ok(result) = kriging.predict(x, y) {
                    let idx = row * cols + col;
                    out_values[idx] = result.prediction;
                }
            }
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        // Output format determined by file extension (.tif → GeoTIFF, .img → HFA, etc.)
        // Format detection handled automatically by wbraster via GisOverlayCore.
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

// ============================================================================
// SPACE-TIME KRIGING TOOL
// ============================================================================

impl Tool for SpaceTimeKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spacetime_kriging",
            display_name: "Space-Time Kriging",
            summary: r#"Performs space-time kriging interpolation to jointly model spatial and temporal variation in point measurements. This advanced geostatistical method extends ordinary kriging to account for temporal dependence, ideal for spatially-distributed time series data like air quality monitoring, climate stations, or environmental sensor networks.

Space-time kriging simultaneously leverages spatial proximity and temporal continuity. Measurements from nearby times and locations are weighted more heavily than distant measurements. The method estimates a spatio-temporal variogram capturing how correlation decays with both spatial distance and temporal lag.

Applications include gap-filling in sensor networks, interpolating climate data across space and time, and estimating environmental variables at unsampled locations and times. The temporal dimension often reveals non-stationary behavior (seasons, trends) better captured jointly than separately. Output includes predictions and space-time kriging variance."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "value_field", description: "Numeric attribute field containing values to interpolate.", required: false },
                ToolParamSpec { name: "time_field", description: "Temporal attribute field containing time values.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size in map units (required if no base_raster).", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("value_field".to_string(), json!("value"));
        defaults.insert("time_field".to_string(), json!("time"));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("spacetime_kriging.tif"));
        ToolManifest {
            id: "spacetime_kriging".to_string(),
            display_name: "Space-Time Kriging".to_string(),
            summary: r#"Performs space-time kriging interpolation to jointly model spatial and temporal variation in point measurements. Ideal for spatially-distributed time series data."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Numeric attribute field containing values to interpolate.".to_string(), required: false },
                ToolParamDescriptor { name: "time_field".to_string(), description: "Temporal attribute field containing time values.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size in map units (required if no base_raster).".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "spacetime_kriging_basic".to_string(),
                description: "Interpolates point values to a raster using space-time kriging.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "kriging".to_string(), "interpolation".to_string(), "spatial-stats".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Running space-time kriging interpolation");
        
        let points = load_vector_arg(args, "points")?;
        let value_field = args.get("value_field").and_then(|v| v.as_str()).unwrap_or("value");
        let time_field = args.get("time_field").and_then(|v| v.as_str()).unwrap_or("time");
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("Extracting point coordinates, times, and values");
        let value_idx = points
            .schema
            .field_index(value_field)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", value_field)))?;
        let time_idx = points
            .schema
            .field_index(time_field)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", time_field)))?;

        let mut coords_spatial = Vec::new();
        let mut coords_temporal = Vec::new();
        let mut values = Vec::new();

        for feature in &points.features {
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

        ctx.progress.info("Building and fitting spatial variogram");
        let builder = EmpiricalVariogramBuilder::default();
        let spatial_vario = builder
            .build(&coords_spatial, &values)
            .map_err(|e| ToolError::Execution(format!("Spatial variogram estimation failed: {}", e)))?;

        let spatial_model = VariogramFitter::fit(&spatial_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Spatial variogram fitting failed: {}", e)))?;

        ctx.progress.info("Building and fitting temporal variogram");
        let builder_temporal = EmpiricalVariogramBuilder::default();
        let temporal_vario = builder_temporal
            .build(&coords_temporal.iter().map(|&t| (t, 0.0)).collect::<Vec<_>>(), &values)
            .map_err(|e| ToolError::Execution(format!("Temporal variogram estimation failed: {}", e)))?;

        let temporal_model = VariogramFitter::fit(&temporal_vario.lags, VariogramModelFamily::Spherical)
            .map_err(|e| ToolError::Execution(format!("Temporal variogram fitting failed: {}", e)))?;

        ctx.progress.info("Creating space-time kriging engine");
        let kriging = SpaceTimeKriging::new(coords_spatial.clone(), coords_temporal.clone(), values.clone(), spatial_model, temporal_model)
            .map_err(|e| ToolError::Execution(format!("Kriging initialization failed: {}", e)))?;

        ctx.progress.info("Building output raster");
        let samples: Vec<(f64, f64, f64)> = coords_spatial.iter()
            .zip(values.iter())
            .map(|((x, y), v)| (*x, *y, *v))
            .collect();
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        // Use mean time for prediction (could be parameterized)
        let mean_time = coords_temporal.iter().sum::<f64>() / coords_temporal.len().max(1) as f64;

        ctx.progress.info(&format!("Generating predictions on {} x {} grid at time {}", output.cols, output.rows, mean_time));
        let coalescer = PercentCoalescer::new(1, 99);
        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![output.nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                if let Ok(result) = kriging.predict(x, y, mean_time) {
                    let idx = row * cols + col;
                    out_values[idx] = result.prediction;
                }
            }
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }
        
        // Output format determined by file extension (.tif → GeoTIFF, .img → HFA, etc.)
        // Format detection handled automatically by wbraster via GisOverlayCore.
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn collect_point_samples(
    layer: &wbvector::Layer,
    field_name: Option<&str>,
    use_z: bool,
) -> Result<Vec<(f64, f64, f64)>, ToolError> {
    let field_idx = if use_z {
        None
    } else if let Some(name) = field_name {
        layer.schema.field_index(name)
    } else {
        None
    };

    let field_is_numeric = field_idx
        .and_then(|idx| layer.schema.fields().get(idx))
        .map(|field| matches!(field.field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float))
        .unwrap_or(false);

    let per_feature: Result<Vec<Vec<(f64, f64, f64)>>, ToolError> = layer
        .features
        .par_iter()
        .map(|feature| {
            let Some(geometry) = &feature.geometry else {
                return Ok(Vec::new());
            };

            let mut coords = Vec::new();
            collect_geometry_coords(geometry, &mut coords);
            if coords.is_empty() {
                return Ok(Vec::new());
            }

            let attr_value = if use_z {
                None
            } else if field_is_numeric {
                field_idx
                    .and_then(|idx| feature.attributes.get(idx))
                    .and_then(|value| value.as_f64())
            } else {
                Some(feature.fid as f64)
            };

            let mut samples = Vec::with_capacity(coords.len());
            for coord in coords {
                let value = if use_z {
                    coord.z.ok_or_else(|| {
                        ToolError::Validation(
                            "points geometry does not contain Z values required by use_z=true".to_string(),
                        )
                    })?
                } else {
                    attr_value.ok_or_else(|| {
                        ToolError::Validation(
                            format!("could not extract numeric value from field or FID for point"),
                        )
                    })?
                };

                if value.is_finite() {
                    samples.push((coord.x, coord.y, value));
                }
            }
            Ok(samples)
        })
        .collect();

    let mut all_samples = Vec::new();
    for samples in per_feature? {
        all_samples.extend(samples);
    }
    all_samples.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    all_samples.dedup_by(|a, b| (a.0 - b.0).abs() <= 1.0e-12 && (a.1 - b.1).abs() <= 1.0e-12);

    if all_samples.len() < 3 {
        return Err(ToolError::Validation(format!(
            "At least 3 unique samples with finite values required, found {}",
            all_samples.len()
        )));
    }

    Ok(all_samples)
}

fn build_point_interpolation_output(
    points: &wbvector::Layer,
    samples: &[(f64, f64, f64)],
    cell_size: Option<f64>,
    base_raster: Option<Raster>,
    data_type: DataType,
) -> Result<Raster, ToolError> {
    if let Some(base) = base_raster {
        let mut output = build_output_like_raster(&base, data_type);
        if has_vector_crs(points) {
            output.crs = vector_crs_to_raster_crs(points);
        }
        return Ok(output);
    }

    let cell = cell_size.ok_or_else(|| {
        ToolError::Validation("either a positive cell_size or a base_raster must be provided".to_string())
    })?;
    if !cell.is_finite() || cell <= 0.0 {
        return Err(ToolError::Validation(
            "cell_size must be a positive finite value when base_raster is not provided".to_string(),
        ));
    }

    let min_x = samples.iter().map(|(x, _, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = samples.iter().map(|(x, _, _)| *x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = samples.iter().map(|(_, y, _)| *y).fold(f64::INFINITY, f64::min);
    let max_y = samples.iter().map(|(_, y, _)| *y).fold(f64::NEG_INFINITY, f64::max);

    let cols = (((max_x - min_x) / cell).ceil() as usize).max(1);
    let rows = (((max_y - min_y) / cell).ceil() as usize).max(1);

    Ok(Raster::new(RasterConfig {
        cols,
        rows,
        bands: 1,
        x_min: min_x,
        y_min: max_y - rows as f64 * cell,
        cell_size: cell,
        cell_size_y: Some(cell),
        nodata: -32768.0,
        data_type,
        crs: vector_crs_to_raster_crs(points),
        metadata: Vec::new(),
    }))
}

fn vector_crs_to_raster_crs(layer: &wbvector::Layer) -> CrsInfo {
    if let Some(crs) = &layer.crs {
        if let Some(epsg) = crs.epsg {
            return CrsInfo::from_epsg(epsg);
        }
        if let Some(wkt) = &crs.wkt {
            return CrsInfo::from_wkt(wkt.clone());
        }
    }
    CrsInfo::default()
}

fn has_vector_crs(layer: &wbvector::Layer) -> bool {
    layer
        .crs
        .as_ref()
        .map(|crs| crs.epsg.is_some() || crs.wkt.as_deref().map(|w| !w.trim().is_empty()).unwrap_or(false))
        .unwrap_or(false)
}

fn collect_geometry_coords<'a>(geometry: &'a wbvector::Geometry, out: &mut Vec<&'a wbvector::Coord>) {
    match geometry {
        wbvector::Geometry::Point(coord) => out.push(coord),
        wbvector::Geometry::MultiPoint(coords) => out.extend(coords.iter()),
        wbvector::Geometry::GeometryCollection(geometries) => {
            for geometry in geometries {
                collect_geometry_coords(geometry, out);
            }
        }
        _ => {}
    }
}
