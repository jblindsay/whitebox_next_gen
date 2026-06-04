//! Phase D point-process and pattern analysis tools
//!
//! Includes Ripley's K/L, envelope testing, inhomogeneous baselines, residual diagnostics, and comparison tools.

use super::*;
use wbspatialstats::point_process::{KFunction, CriticalBandEnvelope, InhomogeneousKProcess, PointProcessResiduals, ResidualType};

/// Computes Ripley's K and L functions for point pattern analysis
pub struct RipleysKFunctionTool;

/// Tests point pattern against complete spatial randomness (CSR) using Monte Carlo envelopes
pub struct PointPatternEnvelopeTool;

/// Estimates intensity surface (λ) and computes intensity-corrected K function
pub struct InhomogeneousBaselineTool;

/// Computes and diagnoses residuals from point process models
pub struct PointProcessResidualsTool;

/// Compares hotspot locations with underlying point-process predictions
pub struct HotspotVsProcessTool;

// Helper function for parsing optional usize
fn parse_opt_usize(args: &ToolArgs, key: &str) -> Result<Option<usize>, ToolError> {
    match args.get(key) {
        None => Ok(None),
        Some(value) => {
            if let Some(n) = value.as_i64() {
                if n > 0 {
                    Ok(Some(n as usize))
                } else {
                    Err(ToolError::Validation(format!("parameter '{}' must be > 0", key)))
                }
            } else {
                Err(ToolError::Validation(format!("parameter '{}' must be an integer", key)))
            }
        }
    }
}

// ============================================================================
// RIPLEY'S K FUNCTION TOOL
// ============================================================================

impl Tool for RipleysKFunctionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ripleys_k_function",
            display_name: "Ripley's K Function",
            summary: "Computes Ripley's K and L functions for point pattern characterization.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "max_distance", description: "Maximum distance for K computation.", required: true },
                ToolParamSpec { name: "num_distances", description: "Number of distance bins (default: 20).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV with K and L values.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.gpkg"));
        defaults.insert("max_distance".to_string(), json!(1.0));
        defaults.insert("num_distances".to_string(), json!(20));
        defaults.insert("output".to_string(), json!("k_function.csv"));

        ToolManifest {
            id: "ripleys_k_function".to_string(),
            display_name: "Ripley's K Function".to_string(),
            summary: "Compute K(t) and L(t) for characterizing spatial clustering patterns.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "max_distance".to_string(), description: "Maximum distance threshold.".to_string(), required: true },
                ToolParamDescriptor { name: "num_distances".to_string(), description: "Number of distance steps.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "k_function_basic".to_string(),
                description: "Compute K function from 0 to 1.0 with 20 distances.".to_string(),
                args: defaults,
            }],
            tags: vec!["vector".to_string(), "point-pattern".to_string(), "spatial-statistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_f64_arg(args, "max_distance")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let max_distance = parse_f64_arg(args, "max_distance")?;
        let num_distances = parse_opt_usize(args, "num_distances")?.unwrap_or(20);
        let output_path = parse_string_arg(args, "output")?;

        ctx.progress.info("Extracting point coordinates");
        let mut points = Vec::new();
        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    points.push((coord.x, coord.y));
                }
            }
        }

        if points.len() < 5 {
            return Err(ToolError::Execution("At least 5 points required".to_string()));
        }

        ctx.progress.info(&format!("Computing K function for {} points", points.len()));
        let kf = KFunction::new(points)
            .map_err(|e| ToolError::Execution(format!("K function creation failed: {}", e)))?;

        let distances: Vec<f64> = (0..num_distances)
            .map(|i| (i as f64 / num_distances as f64) * max_distance)
            .collect();

        let result = kf.compute(&distances)
            .map_err(|e| ToolError::Execution(format!("K computation failed: {}", e)))?;

        ctx.progress.info("Writing output");
        let mut csv = String::from("distance,k_value,l_value\n");
        for i in 0..result.distances.len() {
            csv.push_str(&format!("{},{},{}\n", result.distances[i], result.k_values[i], result.l_values[i]));
        }

        std::fs::write(output_path, csv)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// POINT PATTERN ENVELOPE TOOL
// ============================================================================

impl Tool for PointPatternEnvelopeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "point_pattern_envelope",
            display_name: "Point Pattern Envelope Test",
            summary: "Tests point pattern against CSR using Monte Carlo critical-band envelopes.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "max_distance", description: "Maximum distance for envelope.", required: true },
                ToolParamSpec { name: "num_distances", description: "Number of distance bins (default: 20).", required: false },
                ToolParamSpec { name: "num_simulations", description: "Monte Carlo simulations (default: 99).", required: false },
                ToolParamSpec { name: "alpha", description: "Significance level (default: 0.05).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV with envelope bounds.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.gpkg"));
        defaults.insert("max_distance".to_string(), json!(1.0));
        defaults.insert("num_distances".to_string(), json!(20));
        defaults.insert("num_simulations".to_string(), json!(99));
        defaults.insert("alpha".to_string(), json!(0.05));
        defaults.insert("output".to_string(), json!("envelope.csv"));

        ToolManifest {
            id: "point_pattern_envelope".to_string(),
            display_name: "Point Pattern Envelope Test".to_string(),
            summary: "Generate critical-band envelopes for hypothesis testing against CSR.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "max_distance".to_string(), description: "Maximum distance.".to_string(), required: true },
                ToolParamDescriptor { name: "num_distances".to_string(), description: "Distance bins.".to_string(), required: false },
                ToolParamDescriptor { name: "num_simulations".to_string(), description: "Monte Carlo replicates.".to_string(), required: false },
                ToolParamDescriptor { name: "alpha".to_string(), description: "Significance level.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "envelope_basic".to_string(),
                description: "Test pattern with 99 simulations at α=0.05.".to_string(),
                args: defaults,
            }],
            tags: vec!["vector".to_string(), "hypothesis-test".to_string(), "spatial-statistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_f64_arg(args, "max_distance")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let max_distance = parse_f64_arg(args, "max_distance")?;
        let num_distances = parse_opt_usize(args, "num_distances")?.unwrap_or(20);
        let num_simulations = parse_opt_usize(args, "num_simulations")?.unwrap_or(99);
        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        let output_path = parse_string_arg(args, "output")?;

        ctx.progress.info("Extracting points and computing observed K");
        let mut points = Vec::new();
        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    points.push((coord.x, coord.y));
                }
            }
        }

        if points.len() < 5 {
            return Err(ToolError::Execution("At least 5 points required".to_string()));
        }

        let kf = KFunction::new(points.clone())
            .map_err(|e| ToolError::Execution(format!("K function creation failed: {}", e)))?;

        let distances: Vec<f64> = (0..num_distances)
            .map(|i| (i as f64 / num_distances as f64) * max_distance)
            .collect();

        let result = kf.compute(&distances)
            .map_err(|e| ToolError::Execution(format!("K computation failed: {}", e)))?;

        let min_x = points.iter().map(|(x, _)| x).copied().fold(f64::INFINITY, f64::min);
        let max_x = points.iter().map(|(x, _)| x).copied().fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|(_, y)| y).copied().fold(f64::INFINITY, f64::min);
        let max_y = points.iter().map(|(_, y)| y).copied().fold(f64::NEG_INFINITY, f64::max);
        let bounds = (min_x, min_y, max_x, max_y);
        let intensity = points.len() as f64 / ((max_x - min_x) * (max_y - min_y)).max(1e-10);

        ctx.progress.info(&format!("Generating {} envelope simulations", num_simulations));
        let envelope = CriticalBandEnvelope::generate(
            &result.k_values,
            &result.l_values,
            &distances,
            bounds,
            num_simulations,
            alpha,
            intensity,
        )
        .map_err(|e| ToolError::Execution(format!("Envelope generation failed: {}", e)))?;

        ctx.progress.info("Writing output");
        let mut csv = String::from("distance,observed_k,lower_bound,upper_bound,observed_l,l_lower,l_upper,significant\n");
        for i in 0..envelope.distances.len() {
            let sig = if envelope.is_significant[i] { "yes" } else { "no" };
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                envelope.distances[i],
                envelope.observed_k[i],
                envelope.k_lower[i],
                envelope.k_upper[i],
                envelope.observed_l[i],
                envelope.l_lower[i],
                envelope.l_upper[i],
                sig
            ));
        }

        std::fs::write(output_path, csv)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// INHOMOGENEOUS BASELINE TOOL
// ============================================================================

impl Tool for InhomogeneousBaselineTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "inhomogeneous_baseline",
            display_name: "Inhomogeneous Poisson Process Baseline",
            summary: "Estimates intensity λ(x,y) via KDE and computes intensity-corrected K function.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "max_distance", description: "Maximum distance for K.", required: true },
                ToolParamSpec { name: "num_distances", description: "Number of distance bins (default: 20).", required: false },
                ToolParamSpec { name: "bandwidth", description: "KDE bandwidth for intensity estimation (optional).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV with intensity-corrected K.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.gpkg"));
        defaults.insert("max_distance".to_string(), json!(1.0));
        defaults.insert("num_distances".to_string(), json!(20));
        defaults.insert("output".to_string(), json!("inhom_k.csv"));

        ToolManifest {
            id: "inhomogeneous_baseline".to_string(),
            display_name: "Inhomogeneous Poisson Process Baseline".to_string(),
            summary: "Estimate intensity surface and compute intensity-corrected K function.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "max_distance".to_string(), description: "Maximum distance.".to_string(), required: true },
                ToolParamDescriptor { name: "num_distances".to_string(), description: "Distance bins.".to_string(), required: false },
                ToolParamDescriptor { name: "bandwidth".to_string(), description: "KDE bandwidth (auto-select if omitted).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "inhom_basic".to_string(),
                description: "Compute intensity-corrected K with automatic bandwidth.".to_string(),
                args: defaults,
            }],
            tags: vec!["vector".to_string(), "inhomogeneous".to_string(), "spatial-statistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_f64_arg(args, "max_distance")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let max_distance = parse_f64_arg(args, "max_distance")?;
        let num_distances = parse_opt_usize(args, "num_distances")?.unwrap_or(20);
        let bandwidth = parse_optional_f64_arg(args, "bandwidth");
        let output_path = parse_string_arg(args, "output")?;

        ctx.progress.info("Extracting points");
        let mut points = Vec::new();
        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    points.push((coord.x, coord.y));
                }
            }
        }

        if points.len() < 5 {
            return Err(ToolError::Execution("At least 5 points required".to_string()));
        }

        ctx.progress.info("Creating inhomogeneous K process");
        let process = InhomogeneousKProcess::new(points, bandwidth)
            .map_err(|e| ToolError::Execution(format!("Process creation failed: {}", e)))?;

        let distances: Vec<f64> = (0..num_distances)
            .map(|i| (i as f64 / num_distances as f64) * max_distance)
            .collect();

        ctx.progress.info("Computing intensity-corrected K");
        let result = process.compute_k_inhom(&distances)
            .map_err(|e| ToolError::Execution(format!("K computation failed: {}", e)))?;

        ctx.progress.info("Writing output");
        let mut csv = String::from("distance,k_inhom,l_inhom,intensity_mean,bandwidth\n");
        let mean_intensity = result.intensities.iter().sum::<f64>() / result.intensities.len() as f64;
        for i in 0..result.distances.len() {
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                result.distances[i],
                result.k_values[i],
                result.l_values[i],
                mean_intensity,
                result.bandwidth
            ));
        }

        std::fs::write(output_path, csv)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// POINT PROCESS RESIDUALS TOOL
// ============================================================================

impl Tool for PointProcessResidualsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "point_process_residuals",
            display_name: "Point Process Residuals Diagnostics",
            summary: "Computes and diagnoses residuals from point process models (raw, standardized, Pearson).",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer with observed values.", required: true },
                ToolParamSpec { name: "observed_field", description: "Field name for observed counts/intensities.", required: true },
                ToolParamSpec { name: "predicted_field", description: "Field name for predicted counts/intensities.", required: true },
                ToolParamSpec { name: "residual_type", description: "Type: raw, standardized, or pearson (default: standardized).", required: false },
                ToolParamSpec { name: "output", description: "Output CSV with residuals and diagnostics.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("observations.gpkg"));
        defaults.insert("observed_field".to_string(), json!("observed"));
        defaults.insert("predicted_field".to_string(), json!("predicted"));
        defaults.insert("residual_type".to_string(), json!("standardized"));
        defaults.insert("output".to_string(), json!("residuals.csv"));

        ToolManifest {
            id: "point_process_residuals".to_string(),
            display_name: "Point Process Residuals".to_string(),
            summary: "Compute residual diagnostics for model adequacy checking.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "observed_field".to_string(), description: "Observed values field.".to_string(), required: true },
                ToolParamDescriptor { name: "predicted_field".to_string(), description: "Predicted values field.".to_string(), required: true },
                ToolParamDescriptor { name: "residual_type".to_string(), description: "Residual type.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "residuals_basic".to_string(),
                description: "Compute standardized residuals with diagnostics.".to_string(),
                args: defaults,
            }],
            tags: vec!["vector".to_string(), "diagnostics".to_string(), "spatial-statistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "observed_field")?;
        let _ = parse_string_arg(args, "predicted_field")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let observed_field = parse_string_arg(args, "observed_field")?;
        let predicted_field = parse_string_arg(args, "predicted_field")?;
        let residual_type_str = parse_optional_string_arg(args, "residual_type").unwrap_or("standardized");
        let output_path = parse_string_arg(args, "output")?;

        let residual_type = match residual_type_str.to_lowercase().as_str() {
            "raw" => ResidualType::Raw,
            "pearson" => ResidualType::Pearson,
            _ => ResidualType::Standardized,
        };

        ctx.progress.info("Extracting observed and predicted values");
        let obs_idx = input.schema.field_index(observed_field)
            .ok_or_else(|| ToolError::Validation(format!("Field '{}' not found", observed_field)))?;
        let pred_idx = input.schema.field_index(predicted_field)
            .ok_or_else(|| ToolError::Validation(format!("Field '{}' not found", predicted_field)))?;

        let mut locations = Vec::new();
        let mut observed = Vec::new();
        let mut predicted = Vec::new();

        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    if let (Some(obs_val), Some(pred_val)) = (
                        feature.attributes.get(obs_idx).and_then(|v| v.as_f64()),
                        feature.attributes.get(pred_idx).and_then(|v| v.as_f64()),
                    ) {
                        if obs_val.is_finite() && pred_val.is_finite() {
                            locations.push((coord.x, coord.y));
                            observed.push(obs_val);
                            predicted.push(pred_val);
                        }
                    }
                }
            }
        }

        if locations.is_empty() {
            return Err(ToolError::Execution("No valid observations found".to_string()));
        }

        ctx.progress.info("Computing residuals");
        let residuals = PointProcessResiduals::compute(locations, observed, predicted, residual_type)
            .map_err(|e| ToolError::Execution(format!("Residual computation failed: {}", e)))?;

        let (is_adequate, diagnostics) = residuals.adequacy_check();

        ctx.progress.info(&diagnostics);

        ctx.progress.info("Writing output");
        let mut csv = String::from("x,y,observed,predicted,residual,deviance\n");
        for i in 0..residuals.locations.len() {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                residuals.locations[i].0,
                residuals.locations[i].1,
                residuals.observed[i],
                residuals.predicted[i],
                residuals.residuals[i],
                residuals.deviance[i]
            ));
        }

        csv.push_str("\nDiagnostics:\n");
        csv.push_str(&format!("Total Deviance: {}\n", residuals.total_deviance));
        csv.push_str(&format!("AIC: {}\n", residuals.aic));
        csv.push_str(&format!("Model Adequate: {}\n", if is_adequate { "YES" } else { "NO" }));

        std::fs::write(output_path, csv)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        outputs.insert("adequate".to_string(), json!(is_adequate));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// HOTSPOT VS PROCESS COMPARISON TOOL
// ============================================================================

impl Tool for HotspotVsProcessTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "hotspot_vs_process",
            display_name: "Hotspot vs Process Comparison",
            summary: "Compares spatial hotspot classifications with point-process model predictions.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector with hotspot classifications.", required: true },
                ToolParamSpec { name: "hotspot_field", description: "Field with hotspot classification (hot/cold/insignificant).", required: true },
                ToolParamSpec { name: "intensity_field", description: "Field with estimated intensity λ.", required: true },
                ToolParamSpec { name: "output", description: "Output CSV with comparison metrics.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("classified.gpkg"));
        defaults.insert("hotspot_field".to_string(), json!("hotspot_class"));
        defaults.insert("intensity_field".to_string(), json!("intensity"));
        defaults.insert("output".to_string(), json!("comparison.csv"));

        ToolManifest {
            id: "hotspot_vs_process".to_string(),
            display_name: "Hotspot vs Process Comparison".to_string(),
            summary: "Compare hotspot patterns with underlying point-process intensity.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "hotspot_field".to_string(), description: "Hotspot classification field.".to_string(), required: true },
                ToolParamDescriptor { name: "intensity_field".to_string(), description: "Intensity field.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output CSV.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "hotspot_comparison".to_string(),
                description: "Compare Getis-Ord hotspots with KDE intensity.".to_string(),
                args: defaults,
            }],
            tags: vec!["vector".to_string(), "comparison".to_string(), "spatial-statistics".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "hotspot_field")?;
        let _ = parse_string_arg(args, "intensity_field")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let hotspot_field = parse_string_arg(args, "hotspot_field")?;
        let intensity_field = parse_string_arg(args, "intensity_field")?;
        let output_path = parse_string_arg(args, "output")?;

        ctx.progress.info("Extracting hotspot and intensity data");
        let hotspot_idx = input.schema.field_index(hotspot_field)
            .ok_or_else(|| ToolError::Validation(format!("Field '{}' not found", hotspot_field)))?;
        let intensity_idx = input.schema.field_index(intensity_field)
            .ok_or_else(|| ToolError::Validation(format!("Field '{}' not found", intensity_field)))?;

        let mut hot_points = 0;
        let mut cold_points = 0;
        let mut insignificant_points = 0;
        let mut hot_intensity_sum = 0.0;
        let mut cold_intensity_sum = 0.0;
        let mut insignificant_intensity_sum = 0.0;
        let mut hot_intensity_count = 0;
        let mut cold_intensity_count = 0;
        let mut insignificant_intensity_count = 0;

        let mut csv = String::from("hotspot_class,intensity\n");

        for feature in &input.features {
            if let (Some(hotspot_val), Some(intensity_val)) = (
                feature.attributes.get(hotspot_idx).and_then(|v| v.as_str()),
                feature.attributes.get(intensity_idx).and_then(|v| v.as_f64()),
            ) {
                let class = hotspot_val.trim().to_lowercase();
                csv.push_str(&format!("{},{}\n", class, intensity_val));

                match class.as_str() {
                    "hot" | "hot spot" => {
                        hot_points += 1;
                        hot_intensity_sum += intensity_val;
                        hot_intensity_count += 1;
                    }
                    "cold" | "cold spot" => {
                        cold_points += 1;
                        cold_intensity_sum += intensity_val;
                        cold_intensity_count += 1;
                    }
                    _ => {
                        insignificant_points += 1;
                        insignificant_intensity_sum += intensity_val;
                        insignificant_intensity_count += 1;
                    }
                }
            }
        }

        ctx.progress.info("Computing comparison metrics");
        let hot_mean = if hot_intensity_count > 0 { hot_intensity_sum / hot_intensity_count as f64 } else { 0.0 };
        let cold_mean = if cold_intensity_count > 0 { cold_intensity_sum / cold_intensity_count as f64 } else { 0.0 };
        let insignificant_mean = if insignificant_intensity_count > 0 { 
            insignificant_intensity_sum / insignificant_intensity_count as f64 
        } else { 
            0.0 
        };

        ctx.progress.info("Writing output");
        csv.push_str("\n=== Summary Statistics ===\n");
        csv.push_str(&format!("Hot Spots: {} (mean intensity: {:.4})\n", hot_points, hot_mean));
        csv.push_str(&format!("Cold Spots: {} (mean intensity: {:.4})\n", cold_points, cold_mean));
        csv.push_str(&format!("Insignificant: {} (mean intensity: {:.4})\n", insignificant_points, insignificant_mean));
        csv.push_str(&format!("\nHotspot/Coldspot Intensity Ratio: {:.4}\n", 
            if cold_mean > 0.0 { hot_mean / cold_mean } else { 0.0 }));

        std::fs::write(output_path, csv)
            .map_err(|e| ToolError::Execution(format!("Write failed: {}", e)))?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(output_path));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}
