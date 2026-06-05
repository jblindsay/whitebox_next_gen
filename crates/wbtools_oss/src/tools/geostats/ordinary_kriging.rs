use super::*;
use wbraster::{Raster, RasterFormat, raster::RasterData};
use wbspatialstats::variogram::directional::AnisotropyModel;

#[allow(dead_code)]
pub struct OrdinaryKrigingTool;

impl Tool for OrdinaryKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ordinary_kriging",
            display_name: "Ordinary Kriging Interpolation",
            summary: r#"Performs kriging-based spatial interpolation from point observations to a regular grid: estimates values at unsampled locations using weighted linear combination of nearby observed values. Ordinary kriging assumes an unknown constant mean and automatically determines weights from empirical variogram structure, producing both predictions and kriging variance (prediction uncertainty).

Kriging is optimal for continuous spatial data when spatial correlation structure (variogram) is known. Unlike inverse-distance weighting, kriging incorporates spatial autocorrelation, produces unbiased predictions, and provides uncertainty estimates. Workflow: estimate variogram from points → fit theoretical model → validate with cross-validation → perform kriging on grid.

Ordinary kriging is most general-purpose variant (stationary random function, constant but unknown mean). Optional prediction intervals quantify confidence; optional anisotropy accommodates directional correlation patterns (e.g., geological layering, fracture trends). Output: predictions raster + optional kriging variance raster ± confidence bounds."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "training_points", description: "Training point layer", required: true },
                ToolParamSpec { name: "field", description: "Field with values", required: true },
                ToolParamSpec { name: "variogram_json", description: "Fitted variogram JSON", required: true },
                ToolParamSpec { name: "template_raster", description: "Template raster defining grid", required: true },
                ToolParamSpec { name: "output", description: "Output kriged raster", required: true },
                ToolParamSpec { name: "output_intervals", description: "Compute prediction intervals", required: false },
                ToolParamSpec { name: "confidence_level", description: "Confidence level (0.8-0.99)", required: false },
                ToolParamSpec { name: "interval_method", description: "Interval method: gaussian or posterior", required: false },
                ToolParamSpec { name: "anisotropy", description: "Enable anisotropic distance metric", required: false },
                ToolParamSpec { name: "major_azimuth", description: "Azimuth of maximum continuity (0-180)", required: false },
                ToolParamSpec { name: "anisotropy_ratio", description: "Anisotropy ratio (minor/major range, 0-1)", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("training_points".to_string(), json!("samples.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("variogram_json".to_string(), json!("{}"));
        defaults.insert("template_raster".to_string(), json!("template.tif"));
        defaults.insert("output".to_string(), json!("kriged.tif"));
        defaults.insert("output_intervals".to_string(), json!(false));
        defaults.insert("confidence_level".to_string(), json!(0.95));
        defaults.insert("interval_method".to_string(), json!("gaussian"));
        defaults.insert("anisotropy".to_string(), json!(false));
        defaults.insert("major_azimuth".to_string(), json!(0.0));
        defaults.insert("anisotropy_ratio".to_string(), json!(1.0));

        let mut example_intervals = defaults.clone();
        example_intervals.insert("output_intervals".to_string(), json!(true));
        example_intervals.insert("confidence_level".to_string(), json!(0.90));

        let mut example_anisotropy = defaults.clone();
        example_anisotropy.insert("anisotropy".to_string(), json!(true));
        example_anisotropy.insert("major_azimuth".to_string(), json!(45.0));
        example_anisotropy.insert("anisotropy_ratio".to_string(), json!(0.6));

        ToolManifest {
            id: "ordinary_kriging".to_string(),
            display_name: "Ordinary Kriging Interpolation".to_string(),
            summary: "Performs kriging-based spatial interpolation from point observations to a regular grid: estimates values at unsampled locations using weighted linear combination of nearby observed values. Ordinary kriging assumes an unknown constant mean and automatically determines weights from empirical variogram structure, producing both predictions and kriging variance (prediction uncertainty).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "training_points".to_string(), description: "Vector layer with training points".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Field containing measurement values".to_string(), required: true },
                ToolParamDescriptor { name: "variogram_json".to_string(), description: "Fitted variogram model as JSON".to_string(), required: true },
                ToolParamDescriptor { name: "template_raster".to_string(), description: "Raster template defining output grid and CRS".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output kriged raster path".to_string(), required: true },
                ToolParamDescriptor { name: "output_intervals".to_string(), description: "If true, output additional rasters with confidence interval bounds".to_string(), required: false },
                ToolParamDescriptor { name: "confidence_level".to_string(), description: "Confidence level for prediction intervals (0.80-0.99)".to_string(), required: false },
                ToolParamDescriptor { name: "interval_method".to_string(), description: "Method for intervals: 'gaussian' or 'posterior'".to_string(), required: false },
                ToolParamDescriptor { name: "anisotropy".to_string(), description: "If true, use anisotropic distance metric in kriging".to_string(), required: false },
                ToolParamDescriptor { name: "major_azimuth".to_string(), description: "Direction of maximum continuity (0-180 degrees)".to_string(), required: false },
                ToolParamDescriptor { name: "anisotropy_ratio".to_string(), description: "Ratio of minor to major range (0.0-1.0)".to_string(), required: false },
            ],
            defaults,
            examples: vec![
                ToolExample {
                    name: "ordinary_kriging_basic".to_string(),
                    description: "Basic kriging with point predictions".to_string(),
                    args: example_intervals.clone(),
                },
                ToolExample {
                    name: "ordinary_kriging_intervals".to_string(),
                    description: "Kriging with 90% prediction intervals".to_string(),
                    args: example_intervals,
                },
                ToolExample {
                    name: "ordinary_kriging_anisotropic".to_string(),
                    description: "Anisotropic kriging with 45° major continuity direction".to_string(),
                    args: example_anisotropy,
                },
            ],
            tags: vec!["geostatistics".to_string(), "kriging".to_string(), "raster".to_string(), "interpolation".to_string(), "uncertainty".to_string(), "anisotropy".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "training_points")?;
        let _field = parse_string_arg(args, "field")?;
        let _vario_json = parse_string_arg(args, "variogram_json")?;
        let _template = parse_string_arg(args, "template_raster")?;
        let _output = parse_string_arg(args, "output")?;
        
        // Validate optional parameters
        let output_intervals = parse_bool_arg(args, "output_intervals", false);
        let confidence = parse_optional_f64_arg(args, "confidence_level").unwrap_or(0.95);
        if output_intervals && (confidence <= 0.5 || confidence >= 1.0) {
            return Err(ToolError::Validation(
                "confidence_level must be in (0.5, 1.0) when output_intervals=true".to_string()
            ));
        }
        
        let interval_method = args.get("interval_method")
            .and_then(|v| v.as_str())
            .unwrap_or("gaussian")
            .to_ascii_lowercase();
        if output_intervals && interval_method != "gaussian" && interval_method != "posterior" {
            return Err(ToolError::Validation(
                "interval_method must be 'gaussian' or 'posterior'".to_string()
            ));
        }
        
        let anisotropy = parse_bool_arg(args, "anisotropy", false);
        if anisotropy {
            let azimuth = parse_optional_f64_arg(args, "major_azimuth").unwrap_or(0.0);
            let ratio = parse_optional_f64_arg(args, "anisotropy_ratio").unwrap_or(1.0);
            if azimuth < 0.0 || azimuth > 180.0 {
                return Err(ToolError::Validation("major_azimuth must be in [0, 180]".to_string()));
            }
            if ratio <= 0.0 || ratio > 1.0 {
                return Err(ToolError::Validation("anisotropy_ratio must be in (0, 1]".to_string()));
            }
        }
        
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Ordinary Kriging Interpolation (Raster)");
        
        let training = load_vector_arg(args, "training_points")?;
        let field_name = parse_string_arg(args, "field")?;
        let vario_json_str = parse_string_arg(args, "variogram_json")?;
        let template_path = parse_string_arg(args, "template_raster")?;
        let output_path = parse_string_arg(args, "output")?;
        
        // New parameters
        let output_intervals = parse_bool_arg(args, "output_intervals", false);
        let confidence_level = parse_optional_f64_arg(args, "confidence_level").unwrap_or(0.95);
        let interval_method = args.get("interval_method")
            .and_then(|v| v.as_str())
            .unwrap_or("gaussian")
            .to_ascii_lowercase();
        let anisotropy = parse_bool_arg(args, "anisotropy", false);
        let major_azimuth = parse_optional_f64_arg(args, "major_azimuth").unwrap_or(0.0);
        let anisotropy_ratio = parse_optional_f64_arg(args, "anisotropy_ratio").unwrap_or(1.0);

        // Parse variogram JSON
        let vario_obj: Value = serde_json::from_str(&vario_json_str)
            .map_err(|e| ToolError::Execution(format!("Variogram JSON parse error: {}", e)))?;

        let family_str = vario_obj.get("family")
            .and_then(|v| v.as_str())
            .unwrap_or("exponential");
        
        let family = match family_str {
            "spherical" => VariogramModelFamily::Spherical,
            "exponential" => VariogramModelFamily::Exponential,
            "gaussian" => VariogramModelFamily::Gaussian,
            _ => return Err(ToolError::Execution("Invalid variogram family".to_string())),
        };

        let nugget = vario_obj.get("nugget").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let partial_sill = vario_obj.get("partial_sill").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let range = vario_obj.get("range").and_then(|v| v.as_f64()).unwrap_or(100.0);
        let wrss = vario_obj.get("wrss").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let condition_number = vario_obj.get("condition_number").and_then(|v| v.as_f64()).unwrap_or(1.0);

        let vario = wbspatialstats::variogram::VariogramModel {
            family,
            nugget,
            partial_sill,
            range,
            wrss,
            condition_number,
        };

        // Extract training points
        ctx.progress.info("Loading training points...");
        let field_idx = training.schema.field_index(&field_name)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", field_name)))?;

        let mut coords = Vec::new();
        let mut values = Vec::new();

        for feature in &training.features {
            if let Some(fv) = feature.attributes.get(field_idx) {
                if let Some(value) = fv.as_f64() {
                    if value.is_finite() {
                        if let Some(geom) = &feature.geometry {
                            match geom {
                                wbvector::Geometry::Point(p) => {
                                    coords.push((p.x, p.y));
                                    values.push(value);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if coords.len() < 3 {
            return Err(ToolError::Execution(
                "At least 3 training points required for kriging".to_string()
            ));
        }

        ctx.progress.info(&format!("Loaded {} training points", coords.len()));

        // Load template raster
        ctx.progress.info("Loading template raster...");
        let mut template = Raster::read(template_path)
            .map_err(|e| ToolError::Execution(format!("Failed to read template raster: {}", e)))?;

        ctx.progress.info(&format!("Template grid: {} x {} cells", template.rows, template.cols));

        // Compute residual std for posterior intervals if needed (before moving values)
        let residual_std = if output_intervals && interval_method == "posterior" {
            let mean_val = values.iter().sum::<f64>() / values.len() as f64;
            (values.iter().map(|v| (v - mean_val).powi(2)).sum::<f64>() / values.len() as f64).sqrt()
        } else {
            0.0
        };

        // Build kriging engine
        ctx.progress.info("Building kriging system...");
        
        // Create anisotropy model if needed
        let anisotropy_model = if anisotropy {
            ctx.progress.info(&format!(
                "Applying anisotropic distance: azimuth={:.1}°, ratio={:.3}",
                major_azimuth, anisotropy_ratio
            ));
            Some(AnisotropyModel {
                major_azimuth,
                major_range: 0.0,  // Not used in distance calculation
                minor_range: 0.0,  // Not used in distance calculation
                ratio: anisotropy_ratio,
                angle_tolerance: 22.5,  // Default tolerance for directional analysis
                method: "kriging".to_string(),
            })
        } else {
            None
        };

        // Transform coordinates to anisotropic space if needed
        let transformed_coords = if let Some(ref anis) = anisotropy_model {
            coords.iter()
                .map(|(x, y)| {
                    let dx = *x;
                    let dy = *y;
                    // Apply anisotropic distance transformation
                    let az_rad = anis.major_azimuth * std::f64::consts::PI / 180.0;
                    let cos_az = az_rad.cos();
                    let sin_az = az_rad.sin();
                    
                    // Rotate
                    let x_rot = dx * cos_az + dy * sin_az;
                    let y_rot = -dx * sin_az + dy * cos_az;
                    
                    // Scale
                    let x_scaled = x_rot;
                    let y_scaled = y_rot / anis.ratio;
                    
                    (x_scaled, y_scaled)
                })
                .collect::<Vec<_>>()
        } else {
            coords.clone()
        };

        let kriging = OrdinaryKriging::new(transformed_coords, values, vario)
            .map_err(|e| ToolError::Execution(format!("Kriging setup error: {}", e)))?;

        // Extract grid coordinates from template raster (parallelized generation)
        ctx.progress.info("Generating prediction grid...");
        let grid_coords = generate_raster_grid(&template);
        
        // Transform grid coordinates to anisotropic space if needed
        let transformed_grid = if let Some(ref anis) = anisotropy_model {
            grid_coords.iter()
                .map(|(x, y)| {
                    let dx = *x;
                    let dy = *y;
                    // Apply anisotropic distance transformation
                    let az_rad = anis.major_azimuth * std::f64::consts::PI / 180.0;
                    let cos_az = az_rad.cos();
                    let sin_az = az_rad.sin();
                    
                    // Rotate
                    let x_rot = dx * cos_az + dy * sin_az;
                    let y_rot = -dx * sin_az + dy * cos_az;
                    
                    // Scale
                    let x_scaled = x_rot;
                    let y_scaled = y_rot / anis.ratio;
                    
                    (x_scaled, y_scaled)
                })
                .collect::<Vec<_>>()
        } else {
            grid_coords.clone()
        };
        
        ctx.progress.info(&format!("Predicting {} grid cells...", transformed_grid.len()));

        // Parallel batch prediction - uses rayon internally
        let predictions = kriging.predict_batch(&transformed_grid)
            .map_err(|e| ToolError::Execution(format!("Kriging prediction error: {}", e)))?;

        // Create output raster with predictions
        ctx.progress.info("Building output raster...");
        
        // Replace template data with kriging predictions
        let mut output_data = vec![0.0; template.data.len()];
        let mut output_lower = vec![0.0; template.data.len()];
        let mut output_upper = vec![0.0; template.data.len()];
        
        // Map predictions to raster grid (band-major, then row-major order)
        for (idx, result) in predictions.iter().enumerate() {
            if idx < output_data.len() {
                output_data[idx] = result.prediction;
                
                // Compute prediction intervals if requested
                if output_intervals {
                    let interval = if interval_method == "posterior" {
                        // Posterior includes measurement uncertainty
                        wbspatialstats::kriging::kriging_prediction_interval_posterior(
                            result.prediction,
                            result.variance,
                            residual_std,
                            confidence_level,
                        )
                    } else {
                        // Gaussian: standard Normal-based interval
                        wbspatialstats::kriging::kriging_prediction_interval_gaussian(
                            result.prediction,
                            result.variance,
                            confidence_level,
                        )
                    }.map_err(|e| ToolError::Execution(format!("Interval computation error: {}", e)))?;
                    
                    output_lower[idx] = interval.lower;
                    output_upper[idx] = interval.upper;
                }
            }
        }
        
        template.data = RasterData::F64(output_data);

        // Write output raster
        ctx.progress.info(&format!("Writing output to {}", output_path));
        let format = RasterFormat::for_output_path(output_path)
            .map_err(|e| ToolError::Execution(format!("Invalid output format: {}", e)))?;
        
        template.write(output_path, format)
            .map_err(|e| ToolError::Execution(format!("Failed to write raster: {}", e)))?;

        // Write interval rasters if requested
        if output_intervals {
            let lower_path = output_path.replace(".tif", "_lower.tif").replace(".TIF", "_lower.TIF");
            let upper_path = output_path.replace(".tif", "_upper.tif").replace(".TIF", "_upper.TIF");
            
            ctx.progress.info(&format!("Writing prediction interval bounds..."));
            
            template.data = RasterData::F64(output_lower);
            template.write(&lower_path, format)
                .map_err(|e| ToolError::Execution(format!("Failed to write lower bound raster: {}", e)))?;
            
            template.data = RasterData::F64(output_upper);
            template.write(&upper_path, format)
                .map_err(|e| ToolError::Execution(format!("Failed to write upper bound raster: {}", e)))?;
            
            ctx.progress.info(&format!("Wrote interval bounds to {} and {}", lower_path, upper_path));
        }

        let mut outputs = BTreeMap::new();
        outputs.insert(
            "kriging_report".to_string(),
            json!({
                "training_points": kriging.training_coords.len(),
                "grid_cells": grid_coords.len(),
                "output_path": output_path,
                "output_intervals": output_intervals,
                "confidence_level": if output_intervals { Some(confidence_level) } else { None },
                "interval_method": if output_intervals { Some(&interval_method) } else { None },
                "anisotropy_enabled": anisotropy,
                "status": "complete"
            }),
        );

        ctx.progress.info("Ordinary Kriging interpolation complete");

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}

/// Generate grid coordinates from raster template
/// Uses rayon for parallel coordinate generation
#[allow(dead_code)]
fn generate_raster_grid(raster: &Raster) -> Vec<(f64, f64)> {
    use rayon::prelude::*;

    let rows = raster.rows;
    let cols = raster.cols;
    let x_min = raster.x_min;
    let y_min = raster.y_min;
    let cell_size_x = raster.cell_size_x;
    let cell_size_y = raster.cell_size_y;

    // Generate all (row, col) pairs and convert to (x, y) coordinates
    // Raster origin is top-left (x_min, y_max), grid extends right and down
    (0..rows)
        .into_par_iter()
        .flat_map(move |row| {
            (0..cols).into_par_iter().map(move |col| {
                // Convert raster (row, col) to geographic (x, y)
                // x increases to the right, y decreases downward
                let x = x_min + (col as f64 + 0.5) * cell_size_x;
                let y = y_min + (row as f64 + 0.5) * cell_size_y; // y_min is south edge, y increases upward
                (x, y)
            })
        })
        .collect()
}

