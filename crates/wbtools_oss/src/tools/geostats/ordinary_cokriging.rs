// Ordinary CoKriging Tool
//
// Multi-variate kriging using auxiliary variables to improve primary predictions
// Leverages spatial correlation between variables via cross-variograms
//
// Phase 3 Week 8+ Tool Implementation (2026-06-04)

use super::*;
use wbraster::{Raster, RasterFormat};
use wbspatialstats::variogram::{
    EmpiricalVariogramBuilder, VariogramFitter, VariogramModelFamily, 
    compute_cross_variogram, fit_cross_variogram_model,
};
use wbspatialstats::kriging::OrdinaryCoKriging;

pub struct OrdinaryCoKrigingTool;

impl Tool for OrdinaryCoKrigingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ordinary_cokriging",
            display_name: "Ordinary CoKriging Interpolation",
            summary: r#"Performs multivariate spatial interpolation using auxiliary variables to improve predictions of a primary variable. Ordinary cokriging extends kriging by leveraging correlations between the primary variable of interest and secondary variables (auxiliary data) available at more locations. This produces lower kriging variance and more accurate predictions than using primary data alone.\n\nCokriging is especially valuable when primary variable samples are sparse but correlated secondary data are abundant. Example: estimating soil contamination (sparse primary data) using auxiliary variables like mineral content (more widely measured), topography (universal), or remote sensing indices (complete coverage). The method automatically estimates cross-variograms capturing variable correlations.\n\nRequires fitting both primary variogram and cross-variogram models. Local neighborhood cokriging (k-nearest) is typically more practical than global cokriging for large datasets. Output includes predictions and kriging variance showing prediction confidence by location."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "primary_points", description: "Primary variable training points", required: true },
                ToolParamSpec { name: "primary_field", description: "Field with primary variable values", required: true },
                ToolParamSpec { name: "auxiliary_inputs", description: "Auxiliary variable inputs (comma-separated)", required: true },
                ToolParamSpec { name: "auxiliary_fields", description: "Fields for auxiliary variables (comma-separated)", required: true },
                ToolParamSpec { name: "template_raster", description: "Template raster defining grid", required: true },
                ToolParamSpec { name: "output", description: "Output kriged raster", required: true },
                ToolParamSpec { name: "output_variance", description: "Output variance raster (optional)", required: false },
                ToolParamSpec { name: "neighborhood_size", description: "Number of neighbors for local cokriging", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("primary_points".to_string(), json!("primary.gpkg"));
        defaults.insert("primary_field".to_string(), json!("value"));
        defaults.insert("auxiliary_inputs".to_string(), json!("aux1.gpkg,aux2.tif"));
        defaults.insert("auxiliary_fields".to_string(), json!("value,"));
        defaults.insert("template_raster".to_string(), json!("template.tif"));
        defaults.insert("output".to_string(), json!("cokriged.tif"));
        defaults.insert("output_variance".to_string(), json!(""));
        defaults.insert("neighborhood_size".to_string(), json!(-1));

        ToolManifest {
            id: "ordinary_cokriging".to_string(),
            display_name: "Ordinary CoKriging Interpolation".to_string(),
            summary: r#"Performs multivariate spatial interpolation using auxiliary variables to improve primary variable predictions. Ideal when primary data are sparse but correlated secondary data are abundant."#.to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "primary_points".to_string(), description: "Vector layer with primary variable training points".to_string(), required: true },
                ToolParamDescriptor { name: "primary_field".to_string(), description: "Field containing primary variable values".to_string(), required: true },
                ToolParamDescriptor { name: "auxiliary_inputs".to_string(), description: "Comma-separated list of auxiliary variable file paths".to_string(), required: true },
                ToolParamDescriptor { name: "auxiliary_fields".to_string(), description: "Comma-separated field names for auxiliary variables (empty for rasters)".to_string(), required: true },
                ToolParamDescriptor { name: "template_raster".to_string(), description: "Raster template defining output grid and CRS".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output kriged raster path".to_string(), required: true },
                ToolParamDescriptor { name: "output_variance".to_string(), description: "Optional output kriging variance raster path".to_string(), required: false },
                ToolParamDescriptor { name: "neighborhood_size".to_string(), description: "Number of nearest neighbors for local cokriging (default: all)".to_string(), required: false },
            ],
            defaults: defaults.clone(),
            examples: vec![
                ToolExample {
                    name: "cokriging_basic".to_string(),
                    description: "Basic cokriging with one auxiliary variable".to_string(),
                    args: defaults,
                },
            ],
            tags: vec!["geostatistics".to_string(), "cokriging".to_string(), "multivariate".to_string(), "raster".to_string(), "interpolation".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "primary_points")?;
        let _ = parse_string_arg(args, "primary_field")?;
        let _ = parse_string_arg(args, "auxiliary_inputs")?;
        let _ = parse_string_arg(args, "auxiliary_fields")?;
        let _ = parse_string_arg(args, "template_raster")?;
        let _ = parse_string_arg(args, "output")?;
        Ok(())
    }

    fn run(
        &self,
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Ordinary CoKriging - Phase 5 Full Workflow Implementation");
        
        // Parse arguments
        let primary_points = load_vector_arg(args, "primary_points")?;
        let primary_field = parse_string_arg(args, "primary_field")?;
        let auxiliary_inputs_str = parse_string_arg(args, "auxiliary_inputs")?;
        let template_path = parse_string_arg(args, "template_raster")?;
        let output_path = parse_string_arg(args, "output")?;
        let output_variance = parse_optional_string_arg(args, "output_variance")?;
        let neighborhood_size_arg = parse_optional_string_arg(args, "neighborhood_size")?;
        let neighborhood_size = neighborhood_size_arg.and_then(|s| s.parse::<usize>().ok());

        // STEP 1: Extract primary training data
        ctx.progress.info("Step 1/8: Loading primary training data...");
        let primary_field_idx = primary_points.schema.field_index(&primary_field)
            .ok_or_else(|| ToolError::Validation(format!("Field '{}' not found in primary points", primary_field)))?;

        let mut primary_coords = Vec::new();
        let mut primary_values = Vec::new();

        for feature in &primary_points.features {
            if let Some(fv) = feature.attributes.get(primary_field_idx) {
                if let Some(value) = fv.as_f64() {
                    if value.is_finite() {
                        if let Some(geom) = &feature.geometry {
                            match geom {
                                wbvector::Geometry::Point(pt) => {
                                    primary_coords.push((pt.x, pt.y));
                                    primary_values.push(value);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if primary_coords.is_empty() {
            return Err(ToolError::Validation("No valid primary training points found".to_string()));
        }

        ctx.progress.info(&format!("  Loaded {} primary training points", primary_coords.len()));

        // STEP 2: Compute primary variogram
        ctx.progress.info("Step 2/8: Computing primary variogram...");
        let lag_distance = 100.0;
        let lag_tolerance = 50.0;
        let max_lag_count = 20;

        let primary_vgm = EmpiricalVariogramBuilder::default()
            .lag_distance(lag_distance)
            .lag_tolerance(lag_tolerance)
            .max_lag_count(max_lag_count)
            .build(&primary_coords, &primary_values)
            .map_err(|e| ToolError::Execution(format!("Primary variogram computation failed: {}", e)))?;

        // STEP 3: Fit primary model
        ctx.progress.info("Step 3/8: Fitting primary variogram model...");
        let primary_model = VariogramFitter::fit(&primary_vgm.lags, VariogramModelFamily::Exponential)
            .map_err(|e| ToolError::Execution(format!("Primary variogram fitting failed: {}", e)))?;

        ctx.progress.info(&format!("  Primary variogram: range={:.2}, sill={:.2}, nugget={:.2}",
            primary_model.range, primary_model.partial_sill, primary_model.nugget));

        // STEP 4: Load auxiliary data
        ctx.progress.info("Step 4/8: Loading auxiliary variables...");
        let auxiliary_files: Vec<&str> = auxiliary_inputs_str.split(',').map(|s| s.trim()).collect();
        let mut auxiliary_values_list = Vec::new();
        let mut auxiliary_models = Vec::new();
        let mut cross_models = Vec::new();

        for aux_file in auxiliary_files.iter().filter(|f| !f.is_empty()) {
            // Try loading as raster first
            if let Ok(aux_raster) = Raster::read(aux_file) {
                ctx.progress.info(&format!("  Loading auxiliary raster: {}", aux_file));
                let mut aux_vals = Vec::new();

                for &(x, y) in &primary_coords {
                    let col = ((x - aux_raster.x_min) / aux_raster.cell_size_x).floor() as isize;
                    let row = ((aux_raster.y_min - y) / (-aux_raster.cell_size_y)).floor() as isize;

                    if col >= 0 && col < aux_raster.cols as isize && row >= 0 && row < aux_raster.rows as isize {
                        let val = aux_raster.get(0, row, col);
                        if val != aux_raster.nodata {
                            aux_vals.push(val);
                        } else {
                            aux_vals.push(f64::NAN);
                        }
                    } else {
                        aux_vals.push(f64::NAN);
                    }
                }
                auxiliary_values_list.push(aux_vals);
            }
        }

        if auxiliary_values_list.is_empty() {
            return Err(ToolError::Validation("No valid auxiliary variables loaded".to_string()));
        }

        // STEP 5: Compute auxiliary and cross-variograms
        ctx.progress.info("Step 5/8: Computing auxiliary and cross-variograms...");

        for (aux_idx, aux_vals) in auxiliary_values_list.iter().enumerate() {
            // Filter out NaN values for variogram computation
            let valid_indices: Vec<usize> = aux_vals.iter().enumerate()
                .filter(|(_, &v)| v.is_finite())
                .map(|(i, _)| i)
                .collect();

            if valid_indices.len() < 2 {
                ctx.progress.info(&format!("Auxiliary variable {} has fewer than 2 valid points, skipping", aux_idx));
                continue;
            }

            let valid_coords: Vec<(f64, f64)> = valid_indices.iter()
                .map(|&i| primary_coords[i])
                .collect();
            let valid_aux_vals: Vec<f64> = valid_indices.iter()
                .map(|&i| aux_vals[i])
                .collect();

            // Compute auxiliary variogram
            let aux_vgm = EmpiricalVariogramBuilder::default()
                .lag_distance(lag_distance)
                .lag_tolerance(lag_tolerance)
                .max_lag_count(max_lag_count)
                .build(&valid_coords, &valid_aux_vals)
                .map_err(|e| ToolError::Execution(format!("Auxiliary {} variogram failed: {}", aux_idx, e)))?;

            let aux_model = VariogramFitter::fit(&aux_vgm.lags, VariogramModelFamily::Exponential)
                .map_err(|e| ToolError::Execution(format!("Auxiliary {} model fitting failed: {}", aux_idx, e)))?;

            auxiliary_models.push(aux_model);

            // Compute cross-variogram
            let primary_tuples: Vec<(f64, f64, f64)> = valid_indices.iter()
                .map(|&i| (primary_coords[i].0, primary_coords[i].1, primary_values[i]))
                .collect();
            let aux_tuples: Vec<(f64, f64, f64)> = valid_indices.iter()
                .map(|&i| (primary_coords[i].0, primary_coords[i].1, aux_vals[i]))
                .collect();

            let cross_vgm = compute_cross_variogram(&primary_tuples, &aux_tuples,
                lag_distance * max_lag_count as f64, lag_distance)
                .map_err(|e| ToolError::Execution(format!("Cross-variogram {} computation failed: {}", aux_idx, e)))?;

            let cross_model = fit_cross_variogram_model(&cross_vgm, VariogramModelFamily::Exponential,
                "primary", &format!("auxiliary_{}", aux_idx))
                .map_err(|e| ToolError::Execution(format!("Cross-variogram {} fitting failed: {}", aux_idx, e)))?;

            cross_models.push(cross_model);
        }

        ctx.progress.info(&format!("  Fitted {} auxiliary variograms and {} cross-variograms",
            auxiliary_models.len(), cross_models.len()));

        // STEP 6: Create CoKriging predictor
        ctx.progress.info("Step 6/8: Creating CoKriging predictor...");
        let cokriging = OrdinaryCoKriging::new(
            primary_model.clone(),
            cross_models,
            auxiliary_models,
            primary_coords.clone(),
            primary_values.clone(),
            auxiliary_values_list.clone(),
        ).map_err(|e| ToolError::Execution(format!("CoKriging initialization failed: {}", e)))?;

        // STEP 7: Load template and predict
        ctx.progress.info("Step 7/8: Loading template raster...");
        let template = Raster::read(&template_path)
            .map_err(|e| ToolError::Execution(format!("Failed to load template: {}", e)))?;

        let mut output_predictions = template.clone();
        let mut output_variances = template.clone();

        ctx.progress.info(&format!("  Predicting on {x}x{y} grid...", x = template.cols, y = template.rows));
        let total_cells = template.rows * template.cols;
        let mut predicted_count = 0;

        for row in 0..template.rows {
            for col in 0..template.cols {
                let x = template.x_min + (col as f64 + 0.5) * template.cell_size_x;
                let y = template.y_min + (row as f64 + 0.5) * (-template.cell_size_y);

                match cokriging.predict((x, y), neighborhood_size) {
                    Ok(pred) => {
                        output_predictions.set(0, row as isize, col as isize, pred.prediction)
                            .map_err(|e| ToolError::Execution(format!("Failed to set prediction: {}", e)))?;
                        if output_variance.is_some() {
                            output_variances.set(0, row as isize, col as isize, pred.variance)
                                .map_err(|e| ToolError::Execution(format!("Failed to set variance: {}", e)))?;
                        }
                        predicted_count += 1;
                    }
                    Err(_) => {
                        output_predictions.set(0, row as isize, col as isize, template.nodata)
                            .map_err(|e| ToolError::Execution(format!("Failed to set nodata: {}", e)))?;
                        if output_variance.is_some() {
                            output_variances.set(0, row as isize, col as isize, template.nodata)
                                .map_err(|e| ToolError::Execution(format!("Failed to set nodata: {}", e)))?;
                        }
                    }
                }
            }

            if row % 50 == 0 && row > 0 {
                let progress_pct = (row as f64 / template.rows as f64) * 100.0;
                ctx.progress.info(&format!("  Progress: {:.1}% ({}/{})", progress_pct, row, template.rows));
            }
        }

        // STEP 8: Write outputs
        ctx.progress.info("Step 8/8: Writing output rasters...");
        let output_format = RasterFormat::for_output_path(&output_path)
            .map_err(|e| ToolError::Validation(format!("Unsupported output format: {}", e)))?;

        output_predictions.write(&output_path, output_format)
            .map_err(|e| ToolError::Execution(format!("Failed to write predictions: {}", e)))?;

        if let Some(variance_path) = &output_variance {
            output_variances.write(variance_path, output_format)
                .map_err(|e| ToolError::Execution(format!("Failed to write variance: {}", e)))?;
        }

        ctx.progress.info(&format!("CoKriging complete: {}/{} cells predicted ({:.1}% success)",
            predicted_count, total_cells, (predicted_count as f64 / total_cells as f64) * 100.0));

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("output".to_string(), json!(output_path));
        if let Some(variance_path) = output_variance {
            outputs.insert("variance".to_string(), json!(variance_path));
        }
        outputs.insert("summary".to_string(), json!({
            "primary_points": primary_coords.len(),
            "predicted_cells": predicted_count,
            "total_cells": total_cells,
            "success_rate": (predicted_count as f64 / total_cells as f64) * 100.0,
            "primary_range": primary_model.range,
            "primary_sill": primary_model.partial_sill,
        }));

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = OrdinaryCoKrigingTool;
        let meta = tool.metadata();
        assert_eq!(meta.id, "ordinary_cokriging");
        assert!(!meta.display_name.is_empty());
    }

    #[test]
    fn test_tool_manifest() {
        let tool = OrdinaryCoKrigingTool;
        let manifest = tool.manifest();
        assert_eq!(manifest.id, "ordinary_cokriging");
        assert!(!manifest.params.is_empty());
    }
}
