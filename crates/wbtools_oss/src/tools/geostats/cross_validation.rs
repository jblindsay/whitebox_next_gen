use super::*;

#[allow(dead_code)]
pub struct KrigingCrossValidationTool;

impl Tool for KrigingCrossValidationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "kriging_cross_validation",
            display_name: "Kriging Cross-Validation",
            summary: r#"Assesses kriging model performance using Leave-One-Out Cross-Validation (LOOCV), a rigorous validation technique that removes each data point sequentially, predicts its value using surrounding points, and compares prediction to observed value. This process quantifies model goodness-of-fit without requiring independent test data, revealing whether the variogram model and kriging parameters are appropriate.

Cross-validation outputs diagnostic statistics: Mean Error (bias check), Mean Absolute Error (average prediction accuracy), RMSE (emphasizes large errors), and standardized errors (indicates whether variance estimates are realistic). A good model has ME near zero (unbiased), small MAE/RMSE, and standardized errors near normal distribution (mean 0, std dev 1).

Interpret results as validation of variogram model fit. Poor CV statistics suggest: incorrect variogram model, inappropriate kriging variant, or outliers. Use CV diagnostics to iteratively refine variogram model before final interpolation. This prevents overfitting and ensures kriging predictions have reliable uncertainty estimates."#,
            category: ToolCategory::Vector,
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
            id: "kriging_cross_validation".to_string(),
            display_name: "Kriging Cross-Validation".to_string(),
            summary: r#"Assesses kriging model performance using Leave-One-Out Cross-Validation, computing diagnostic statistics to validate variogram fit and kriging appropriateness."#.to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "training_points".to_string(), description: "Vector layer with training points".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Field containing measurement values".to_string(), required: true },
                ToolParamDescriptor { name: "variogram_json".to_string(), description: "Fitted variogram model as JSON".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "kriging_cross_validation_example".to_string(),
                description: "Validate kriging model using LOOCV".to_string(),
                args: example_args,
            }],
            tags: vec!["geostatistics".to_string(), "kriging".to_string(), "model-validation".to_string(), "statistics".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "training_points")?;
        let _field = parse_string_arg(args, "field")?;
        let _vario_json = parse_string_arg(args, "variogram_json")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Kriging Cross-Validation (LOOCV)");
        
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

        let vario = wbspatialstats::variogram::VariogramModel {
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

        if coords.len() < 4 {
            return Err(ToolError::Execution(
                "At least 4 training points required for LOOCV".to_string()
            ));
        }

        ctx.progress.info(&format!("Running LOOCV with {} training points", coords.len()));

        // Run LOOCV
        let cv_metrics = LeaveOneOutCV::validate(&coords, &values, &vario)
            .map_err(|e| ToolError::Execution(format!("LOOCV error: {}", e)))?;

        let mut outputs = BTreeMap::new();

        let is_calibrated = cv_metrics.is_well_calibrated();

        outputs.insert(
            "cv_diagnostics".to_string(),
            json!({
                "mean_error": cv_metrics.mean_error,
                "rmse": cv_metrics.rmse,
                "mean_std_error": cv_metrics.mean_std_error,
                "rmsse": cv_metrics.rmsse,
                "correlation": cv_metrics.correlation,
                "sample_size": cv_metrics.sample_size,
                "is_well_calibrated": is_calibrated,
                "summary": cv_metrics.summary()
            }),
        );

        ctx.progress.info(&format!("LOOCV complete: RMSE={:.3}, correlation={:.3}", 
            cv_metrics.rmse, cv_metrics.correlation));

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}
