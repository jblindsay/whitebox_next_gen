use super::*;

pub struct EstimateVariogramTool;

impl Tool for EstimateVariogramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "estimate_variogram",
            display_name: "Estimate Variogram",
            summary: r#"Computes an empirical semivariogram from point observations to characterize spatial correlation structure. The semivariogram quantifies how dissimilar values become with increasing distance, providing the foundation for all kriging and geostatistical inference. This tool bins pairwise differences according to distance and averages them to create the empirical variogram.

The empirical variogram reveals key spatial characteristics: nugget (small-scale variance), sill (maximum variance), and range (distance at which correlation becomes negligible). These properties are essential for fitting theoretical variogram models used in kriging. The tool outputs empirical lags suitable for visualization and modeling with Fit Variogram.

Lag parameters control variogram resolution: lag_distance sets bin size; lag_tolerance allows flexibility in distance binning; max_lag_count limits output size. Typical workflow: estimate variogram, visualize, fit model, validate with cross-validation, then perform kriging interpolation."#,
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector points", required: true },
                ToolParamSpec { name: "field", description: "Field with values", required: true },
                ToolParamSpec { name: "lag_distance", description: "Lag distance (default: 100.0)", required: false },
                ToolParamSpec { name: "lag_tolerance", description: "Lag tolerance (default: 50.0)", required: false },
                ToolParamSpec { name: "max_lag_count", description: "Max lags (default: 20)", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("lag_distance".to_string(), json!(100.0));
        defaults.insert("lag_tolerance".to_string(), json!(50.0));
        defaults.insert("max_lag_count".to_string(), json!(20));

        let example_args = defaults.clone();

        ToolManifest {
            id: "estimate_variogram".to_string(),
            display_name: "Estimate Variogram".to_string(),
            summary: r#"Computes an empirical semivariogram from point observations to characterize spatial correlation structure. Essential first step in geostatistical workflow."#.to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector points layer with values".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Field containing measurement values".to_string(), required: true },
                ToolParamDescriptor { name: "lag_distance".to_string(), description: "Lag bin size in map units (default: 100.0)".to_string(), required: false },
                ToolParamDescriptor { name: "lag_tolerance".to_string(), description: "Tolerance for lag binning (default: 50.0)".to_string(), required: false },
                ToolParamDescriptor { name: "max_lag_count".to_string(), description: "Maximum number of lags to retain (default: 20)".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "estimate_variogram_example".to_string(),
                description: "Estimate variogram from point observations".to_string(),
                args: example_args,
            }],
            tags: vec!["geostatistics".to_string(), "kriging".to_string(), "spatial-statistics".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _field = parse_string_arg(args, "field")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Loading point data");
        
        let input = load_vector_arg(args, "input")?;
        let field_name = parse_string_arg(args, "field")?;
        
        let lag_distance = parse_optional_f64_arg(args, "lag_distance").unwrap_or(100.0);
        let lag_tolerance = parse_optional_f64_arg(args, "lag_tolerance").unwrap_or(50.0);
        let max_lag_count = parse_optional_i64_arg(args, "max_lag_count")
            .map(|v| v as usize)
            .unwrap_or(20);

        let mut coords = Vec::new();
        let mut values = Vec::new();

        // Extract coordinates and field values
        let field_idx = input.schema.field_index(&field_name)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", field_name)))?;

        for feature in &input.features {
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

        if coords.len() < 2 {
            return Err(ToolError::Execution(
                "At least 2 valid point observations required".to_string(),
            ));
        }

        ctx.progress.info(&format!("Computing variogram from {} points", coords.len()));

        // Build empirical variogram
        let vario = EmpiricalVariogramBuilder::default()
            .lag_distance(lag_distance)
            .lag_tolerance(lag_tolerance)
            .max_lag_count(max_lag_count)
            .build(&coords, &values)
            .map_err(|e| ToolError::Execution(format!("Variogram error: {}", e)))?;

        let mut outputs = BTreeMap::new();

        // Serialize variogram lags
        let mut lags_json = Vec::new();
        for lag in &vario.lags {
            lags_json.push(json!({
                "distance": lag.distance,
                "semivariance": lag.semivariance,
                "pair_count": lag.pair_count
            }));
        }

        outputs.insert(
            "variogram_report".to_string(),
            json!({
                "num_lags": vario.lags.len(),
                "max_lag": vario.max_lag,
                "total_pairs": vario.total_pairs,
                "colocated_pairs": vario.colocated_pairs,
                "lags": lags_json,
                "summary": vario.summary()
            }),
        );

        ctx.progress.info("Variogram estimation complete");

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}
