use super::*;

pub struct OrdinaryKrigingTool;

impl Tool for OrdinaryKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ordinary_kriging",
            display_name: "Ordinary Kriging Interpolation",
            summary: "Predicts values at target locations using Ordinary Kriging",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "training_points", description: "Training point layer", required: true },
                ToolParamSpec { name: "field", description: "Field with values", required: true },
                ToolParamSpec { name: "variogram_json", description: "Fitted variogram JSON", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("training_points".to_string(), json!("samples.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("variogram_json".to_string(), json!("{}"));

        let example_args = defaults.clone();

        ToolManifest {
            id: "ordinary_kriging".to_string(),
            display_name: "Ordinary Kriging Interpolation".to_string(),
            summary: "Predicts values using Ordinary Kriging with fitted variogram".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "training_points".to_string(), description: "Vector layer with training points".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Field containing measurement values".to_string(), required: true },
                ToolParamDescriptor { name: "variogram_json".to_string(), description: "Fitted variogram model as JSON".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "ordinary_kriging_example".to_string(),
                description: "Setup kriging interpolation".to_string(),
                args: example_args,
            }],
            tags: vec!["geostatistics".to_string(), "kriging".to_string(), "raster".to_string(), "interpolation".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "training_points")?;
        let _field = parse_string_arg(args, "field")?;
        let _vario_json = parse_string_arg(args, "variogram_json")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Ordinary Kriging Interpolation");
        
        let training = load_vector_arg(args, "training_points")?;
        let field_name = parse_string_arg(args, "field")?;
        let vario_json_str = parse_string_arg(args, "variogram_json")?;

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
        let field_idx = training.schema.field_index(&field_name)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", field_name)))?;

        let mut coords = Vec::new();
        let mut values = Vec::new();

        for feature in &training.features {
            if let Some(fv) = feature.attributes.get(field_idx) {
                if let Some(value) = fv.as_f64() {
                    if value.is_finite() {
                        if let Some(geom) = &feature.geometry {
                            let point = match geom {
                                wbvector::Geometry::Point(p) => Some((p.x, p.y)),
                                _ => None,
                            };
                            if let Some((x, y)) = point {
                                coords.push((x, y));
                                values.push(value);
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

        ctx.progress.info(&format!("Kriging with {} training points", coords.len()));

        let kriging = OrdinaryKriging::new(coords, values, vario)
            .map_err(|e| ToolError::Execution(format!("Kriging setup error: {}", e)))?;

        // Output kriging setup report
        let mut outputs = BTreeMap::new();
        outputs.insert(
            "kriging_report".to_string(),
            json!({
                "training_points": kriging.training_coords.len(),
                "status": "ready for prediction"
            }),
        );

        ctx.progress.info("Ordinary Kriging setup complete");

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}
