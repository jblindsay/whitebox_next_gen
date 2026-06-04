use super::*;
use wbraster::{Raster, RasterFormat, raster::RasterData};

pub struct OrdinaryKrigingTool;

impl Tool for OrdinaryKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ordinary_kriging",
            display_name: "Ordinary Kriging Interpolation",
            summary: "Interpolates raster grid using Ordinary Kriging with parallel prediction",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "training_points", description: "Training point layer", required: true },
                ToolParamSpec { name: "field", description: "Field with values", required: true },
                ToolParamSpec { name: "variogram_json", description: "Fitted variogram JSON", required: true },
                ToolParamSpec { name: "template_raster", description: "Template raster defining grid", required: true },
                ToolParamSpec { name: "output", description: "Output kriged raster", required: true },
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

        let example_args = defaults.clone();

        ToolManifest {
            id: "ordinary_kriging".to_string(),
            display_name: "Ordinary Kriging Interpolation".to_string(),
            summary: "Interpolates raster grid using Ordinary Kriging with fitted variogram".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "training_points".to_string(), description: "Vector layer with training points".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Field containing measurement values".to_string(), required: true },
                ToolParamDescriptor { name: "variogram_json".to_string(), description: "Fitted variogram model as JSON".to_string(), required: true },
                ToolParamDescriptor { name: "template_raster".to_string(), description: "Raster template defining output grid and CRS".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output kriged raster path".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "ordinary_kriging_example".to_string(),
                description: "Interpolate raster grid using kriging".to_string(),
                args: example_args,
            }],
            tags: vec!["geostatistics".to_string(), "kriging".to_string(), "raster".to_string(), "interpolation".to_string(), "parallel".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "training_points")?;
        let _field = parse_string_arg(args, "field")?;
        let _vario_json = parse_string_arg(args, "variogram_json")?;
        let _template = parse_string_arg(args, "template_raster")?;
        let _output = parse_string_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Ordinary Kriging Interpolation (Raster)");
        
        let training = load_vector_arg(args, "training_points")?;
        let field_name = parse_string_arg(args, "field")?;
        let vario_json_str = parse_string_arg(args, "variogram_json")?;
        let template_path = parse_string_arg(args, "template_raster")?;
        let output_path = parse_string_arg(args, "output")?;

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

        let vario = wbgeostats::variogram::VariogramModel {
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

        // Build kriging engine
        ctx.progress.info("Building kriging system...");
        let kriging = OrdinaryKriging::new(coords, values, vario)
            .map_err(|e| ToolError::Execution(format!("Kriging setup error: {}", e)))?;

        // Extract grid coordinates from template raster (parallelized generation)
        ctx.progress.info("Generating prediction grid...");
        let grid_coords = generate_raster_grid(&template);
        
        ctx.progress.info(&format!("Predicting {} grid cells...", grid_coords.len()));

        // Parallel batch prediction - uses rayon internally
        let predictions = kriging.predict_batch(&grid_coords)
            .map_err(|e| ToolError::Execution(format!("Kriging prediction error: {}", e)))?;

        // Create output raster with predictions
        ctx.progress.info("Building output raster...");
        
        // Replace template data with kriging predictions
        let mut output_data = vec![0.0; template.data.len()];
        
        // Map predictions to raster grid (band-major, then row-major order)
        for (idx, result) in predictions.iter().enumerate() {
            if idx < output_data.len() {
                output_data[idx] = result.prediction;
            }
        }
        
        template.data = RasterData::F64(output_data);

        // Write output raster
        ctx.progress.info(&format!("Writing output to {}", output_path));
        let format = RasterFormat::for_output_path(output_path)
            .map_err(|e| ToolError::Execution(format!("Invalid output format: {}", e)))?;
        
        template.write(output_path, format)
            .map_err(|e| ToolError::Execution(format!("Failed to write raster: {}", e)))?;

        let mut outputs = BTreeMap::new();
        outputs.insert(
            "kriging_report".to_string(),
            json!({
                "training_points": kriging.training_coords.len(),
                "grid_cells": grid_coords.len(),
                "output_path": output_path,
                "status": "complete"
            }),
        );

        ctx.progress.info("Ordinary Kriging interpolation complete");

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}

/// Generate grid coordinates from raster template
/// Uses rayon for parallel coordinate generation
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

