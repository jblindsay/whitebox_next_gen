use super::*;

pub struct FitVariogramTool;

impl Tool for FitVariogramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "fit_variogram",
            display_name: "Fit Variogram Model",
            summary: r#"Fits a theoretical variogram model to empirical semivariogram data, capturing the underlying spatial correlation structure. This tool replaces raw empirical estimates with smooth, continuous functions (Spherical, Exponential, or Gaussian models) that define kriging weights. Theoretical models ensure interpolation remains stable and captures spatial dependence realistically.

Variogram fitting is essential for kriging because empirical variograms contain noise and gaps where no point pairs exist at certain distances. Theoretical models smooth these irregularities while preserving key features: nugget (error + micro-scale variance), sill (maximum correlation distance), and range (distance where correlation plateaus). Different models suit different phenomena: Spherical for abrupt changes, Exponential for gradual decay, Gaussian for smooth processes.

Output includes fitted model parameters used by kriging tools. Workflow: estimate variogram → fit model → cross-validate → interpolate with kriging."#,
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "lags_json", description: "Empirical lags as JSON", required: true },
                ToolParamSpec { name: "model_family", description: "Model: spherical, exponential, gaussian", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("lags_json".to_string(), json!("[]"));
        defaults.insert("model_family".to_string(), json!("exponential"));

        let example_args = defaults.clone();

        ToolManifest {
            id: "fit_variogram".to_string(),
            display_name: "Fit Variogram Model".to_string(),
            summary: r#"Fits theoretical variogram model (Spherical, Exponential, Gaussian) to empirical semivariogram data for use in kriging."#.to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "lags_json".to_string(), description: "JSON array of lag bins from estimate_variogram".to_string(), required: true },
                ToolParamDescriptor { name: "model_family".to_string(), description: "Model: spherical, exponential, or gaussian".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "fit_variogram_example".to_string(),
                description: "Fit exponential variogram model".to_string(),
                args: example_args,
            }],
            tags: vec!["geostatistics".to_string(), "kriging".to_string(), "model-fitting".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let lags_str = parse_string_arg(args, "lags_json")?;
        serde_json::from_str::<Vec<Value>>(&lags_str)
            .map_err(|e| ToolError::Validation(format!("Invalid lags JSON: {}", e)))?;
        
        let model_family = args.get("model_family")
            .and_then(|v| v.as_str())
            .unwrap_or("exponential")
            .to_ascii_lowercase();
        
        match model_family.as_str() {
            "spherical" | "exponential" | "gaussian" => Ok(()),
            _ => Err(ToolError::Validation(
                "model_family must be: spherical, exponential, or gaussian".to_string()
            )),
        }
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Parsing empirical variogram lags");
        
        let lags_str = parse_string_arg(args, "lags_json")?;
        let lags_data: Vec<Value> = serde_json::from_str(&lags_str)
            .map_err(|e| ToolError::Execution(format!("JSON parse error: {}", e)))?;

        let model_family_str = args.get("model_family")
            .and_then(|v| v.as_str())
            .unwrap_or("exponential")
            .to_ascii_lowercase();

        let model_family = match model_family_str.as_str() {
            "spherical" => VariogramModelFamily::Spherical,
            "exponential" => VariogramModelFamily::Exponential,
            "gaussian" => VariogramModelFamily::Gaussian,
            _ => return Err(ToolError::Execution("Invalid model family".to_string())),
        };

        // Parse lags
        let mut lags = Vec::new();
        for lag_val in lags_data {
            let obj = lag_val.as_object()
                .ok_or_else(|| ToolError::Execution("Each lag must be an object".to_string()))?;
            
            let distance = obj.get("distance")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::Execution("Lag missing 'distance'".to_string()))?;
            let semivariance = obj.get("semivariance")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::Execution("Lag missing 'semivariance'".to_string()))?;
            let pair_count = obj.get("pair_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;

            lags.push(wbspatialstats::variogram::LagBin { 
                distance, 
                semivariance, 
                pair_count 
            });
        }

        if lags.len() < 3 {
            return Err(ToolError::Execution(
                "At least 3 lag bins required for model fitting".to_string()
            ));
        }

        ctx.progress.info(&format!("Fitting {} variogram model to {} lags", model_family_str, lags.len()));

        // Fit model
        let model = VariogramFitter::fit(&lags, model_family)
            .map_err(|e| ToolError::Execution(format!("Model fitting error: {}", e)))?;

        let mut outputs = BTreeMap::new();

        outputs.insert(
            "variogram_model".to_string(),
            json!({
                "family": match model.family {
                    VariogramModelFamily::Spherical => "spherical",
                    VariogramModelFamily::Exponential => "exponential",
                    VariogramModelFamily::Gaussian => "gaussian",
                },
                "nugget": model.nugget,
                "partial_sill": model.partial_sill,
                "total_sill": model.nugget + model.partial_sill,
                "range": model.range,
                "wrss": model.wrss,
                "condition_number": model.condition_number,
                "summary": model.summary()
            }),
        );

        ctx.progress.info("Variogram model fitting complete");

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}
