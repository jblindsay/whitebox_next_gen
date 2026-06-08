use super::*;
use wbspatialstats::variogram::directional::{
    compute_directional_variogram, fit_anisotropy, DirectionalVariogramBin, AnisotropyModel,
};

pub struct DirectionalVariogramTool;

impl Tool for DirectionalVariogramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "directional_variogram",
            display_name: "Directional Variogram Analysis",
            summary: r#"Computes variograms in multiple directions to detect and quantify spatial anisotropy—directional variation in spatial correlation. Many natural phenomena exhibit anisotropy: geological deposits follow stratification, atmospheric properties align with wind patterns, groundwater flows along gradients. This tool reveals these directional patterns by computing separate variograms for each specified azimuth.

Anisotropy manifests as different ranges, nuggets, or sills in different directions. Omitting anisotropy in kriging introduces systematic bias. The tool computes directional variograms and fits anisotropy models (elliptical) to characterize the pattern. Output includes directional variogram data and optional rose diagram visualization showing spatial continuity direction and magnitude.

Parameters: directions specify azimuths (0-180°) to analyze (common: 0°, 45°, 90°, 135° for cardinal/diagonal); tolerance allows flexibility in direction binning; rose diagram output visualizes anisotropy pattern. Use results to inform universal kriging with anisotropic variogram model."#,
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point layer", required: true },
                ToolParamSpec { name: "field", description: "Field with measurement values", required: true },
                ToolParamSpec { name: "directions", description: "Azimuths to analyze (0-180, comma-separated)", required: true },
                ToolParamSpec { name: "tolerance", description: "Direction tolerance (degrees)", required: false },
                ToolParamSpec { name: "max_distance", description: "Maximum lag distance", required: false },
                ToolParamSpec { name: "bin_size", description: "Lag bin size", required: false },
                ToolParamSpec { name: "output_json", description: "Output directional results as JSON", required: true },
                ToolParamSpec { name: "output_html", description: "Optional HTML rose diagram output path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("samples.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("directions".to_string(), json!("0,45,90,135"));
        defaults.insert("tolerance".to_string(), json!(22.5));
        defaults.insert("max_distance".to_string(), json!(1000.0));
        defaults.insert("bin_size".to_string(), json!(50.0));
        defaults.insert("output_json".to_string(), json!("directional_vgram.json"));
        defaults.insert("output_html".to_string(), json!("directional_vgram_rose.html"));

        ToolManifest {
            id: "directional_variogram".to_string(),
            display_name: "Directional Variogram Analysis".to_string(),
            summary: r#"Computes variograms in multiple directions to detect spatial anisotropy. Reveals directional continuity patterns essential for realistic kriging."#.to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Point layer with measurement values".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Field containing measurement values".to_string(), required: true },
                ToolParamDescriptor { name: "directions".to_string(), description: "Comma-separated list of azimuths (0-180°) to analyze".to_string(), required: true },
                ToolParamDescriptor { name: "tolerance".to_string(), description: "Direction tolerance in degrees (default 22.5)".to_string(), required: false },
                ToolParamDescriptor { name: "max_distance".to_string(), description: "Maximum lag distance to compute (default 1000)".to_string(), required: false },
                ToolParamDescriptor { name: "bin_size".to_string(), description: "Size of lag bins (default 50)".to_string(), required: false },
                ToolParamDescriptor { name: "output_json".to_string(), description: "Output file path for directional variogram results".to_string(), required: true },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional output path for HTML rose diagram visualization".to_string(), required: false },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "directional_variogram_example".to_string(),
                description: "Compute directional variograms at 4 azimuths (0°, 45°, 90°, 135°) with rose diagram".to_string(),
                args: defaults,
            }],
            tags: vec!["geostatistics".to_string(), "variography".to_string(), "anisotropy".to_string(), "directional".to_string(), "visualization".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        if field.trim().is_empty() {
            return Err(ToolError::Validation("field must be non-empty".to_string()));
        }

        let directions_str = parse_string_arg(args, "directions")?;
        let directions: Result<Vec<f64>, _> = directions_str
            .split(',')
            .map(|s| s.trim().parse::<f64>())
            .collect();

        if let Ok(dirs) = directions {
            if dirs.is_empty() {
                return Err(ToolError::Validation("at least one direction must be specified".to_string()));
            }
            for dir in &dirs {
                if !dir.is_finite() || *dir < 0.0 || *dir > 180.0 {
                    return Err(ToolError::Validation(
                        "all directions must be in [0, 180]".to_string(),
                    ));
                }
            }
        } else {
            return Err(ToolError::Validation("directions must be comma-separated numbers".to_string()));
        }

        let tolerance = parse_optional_f64_arg(args, "tolerance").unwrap_or(22.5);
        if !tolerance.is_finite() || tolerance <= 0.0 || tolerance > 90.0 {
            return Err(ToolError::Validation(
                "tolerance must be in (0, 90]".to_string(),
            ));
        }

        let max_distance = parse_optional_f64_arg(args, "max_distance").unwrap_or(1000.0);
        if !max_distance.is_finite() || max_distance <= 0.0 {
            return Err(ToolError::Validation(
                "max_distance must be finite and positive".to_string(),
            ));
        }

        let bin_size = parse_optional_f64_arg(args, "bin_size").unwrap_or(50.0);
        if !bin_size.is_finite() || bin_size <= 0.0 {
            return Err(ToolError::Validation(
                "bin_size must be finite and positive".to_string(),
            ));
        }

        let _ = parse_string_arg(args, "output_json")?;
        if let Some(path_str) = parse_optional_string_arg(args, "output_html")? {
            if path_str.trim().is_empty() {
                return Err(ToolError::Validation("output_html path must be non-empty if provided".to_string()));
            }
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        ctx.progress.info("Computing directional variograms...");

        let input = load_vector_arg(args, "input")?;
        let field_name = parse_string_arg(args, "field")?;
        let directions_str = parse_string_arg(args, "directions")?;
        let tolerance = parse_optional_f64_arg(args, "tolerance").unwrap_or(22.5);
        let max_distance = parse_optional_f64_arg(args, "max_distance").unwrap_or(1000.0);
        let bin_size = parse_optional_f64_arg(args, "bin_size").unwrap_or(50.0);
        let output_path = parse_string_arg(args, "output_json")?;
        let output_html_str = parse_optional_string_arg(args, "output_html")?;
        let output_html = output_html_str.as_ref().map(|s| std::path::PathBuf::from(s.trim()));

        // Parse directions
        let directions: Vec<f64> = directions_str
            .split(',')
            .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
            .collect();

        // Extract point data
        ctx.progress.info("Extracting point samples...");
        let field_idx = input.schema.field_index(field_name)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", field_name)))?;

        let mut samples = Vec::new();
        for feature in &input.features {
            if let Some(fv) = feature.attributes.get(field_idx) {
                if let Some(value) = fv.as_f64() {
                    if value.is_finite() {
                        if let Some(geom) = &feature.geometry {
                            match geom {
                                wbvector::Geometry::Point(p) => {
                                    samples.push((p.x, p.y, value));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if samples.len() < 2 {
            return Err(ToolError::Execution(
                "At least 2 sample points required for variography".to_string(),
            ));
        }

        ctx.progress.info(&format!("Computing variograms for {} samples", samples.len()));

        // Compute directional variograms
        let mut vgram_bins = Vec::new();
        for (idx, direction) in directions.iter().enumerate() {
            let progress_frac = (idx as f64) / (directions.len() as f64);
            ctx.progress.progress(progress_frac);
            ctx.progress.info(&format!("Computing variogram for direction {}", direction));

            match compute_directional_variogram(&samples, *direction, tolerance, max_distance, bin_size) {
                Ok(vgram) => {
                    ctx.progress.info(&format!(
                        "  Direction {}: {} lags, {} pairs mean",
                        direction,
                        vgram.n_lags(),
                        vgram.mean_pairs_per_lag() as usize
                    ));
                    vgram_bins.push(vgram);
                }
                Err(e) => {
                    ctx.progress.info(&format!("  Warning: Failed to compute variogram at {}: {}", direction, e));
                }
            }
        }

        if vgram_bins.is_empty() {
            return Err(ToolError::Execution("No valid directional variograms computed".to_string()));
        }

        ctx.progress.info("Fitting anisotropy model...");
        let anisotropy = fit_anisotropy(&vgram_bins)
            .map_err(|e| ToolError::Execution(format!("Anisotropy fitting error: {}", e)))?;

        // Build output JSON
        let mut vgram_json = Vec::new();
        for (_i, vgram) in vgram_bins.iter().enumerate() {
            let mut lags_json = Vec::new();
            for j in 0..vgram.n_lags() {
                lags_json.push(json!({
                    "lag": vgram.lags[j],
                    "semivariance": vgram.semivariances[j],
                    "count": vgram.counts[j],
                }));
            }

            vgram_json.push(json!({
                "direction": vgram.direction_azimuth,
                "tolerance": vgram.tolerance,
                "lags": lags_json,
                "sill": vgram.sill,
                "nugget": vgram.nugget,
                "max_semivariance": vgram.max_semivariance(),
                "mean_pairs_per_lag": vgram.mean_pairs_per_lag(),
            }));
        }

        let output_json = json!({
            "tool": "directional_variogram",
            "n_samples": samples.len(),
            "directions": directions,
            "tolerance": tolerance,
            "max_distance": max_distance,
            "bin_size": bin_size,
            "variograms": vgram_json,
            "anisotropy": {
                "major_azimuth": anisotropy.major_azimuth,
                "major_range": anisotropy.major_range,
                "minor_range": anisotropy.minor_range,
                "ratio": anisotropy.ratio,
                "is_anisotropic": anisotropy.is_anisotropic(0.95),
                "method": anisotropy.method,
            },
            "recommendation": if anisotropy.is_anisotropic(0.95) {
                format!(
                    "Anisotropy detected: Use kriging with azimuth={}, ratio={}",
                    anisotropy.major_azimuth, anisotropy.ratio
                )
            } else {
                "Isotropic (omnidirectional kriging is appropriate)".to_string()
            },
        });

        // Write output
        ctx.progress.info(&format!("Writing results to {}", output_path));
        let output_str = serde_json::to_string_pretty(&output_json)
            .map_err(|e| ToolError::Execution(format!("JSON serialization failed: {}", e)))?;

        std::fs::write(output_path, output_str)
            .map_err(|e| ToolError::Execution(format!("Failed to write output: {}", e)))?;

        // Generate rose diagram HTML if requested
        if let Some(html_path) = &output_html {
            ctx.progress.info("Generating rose diagram visualization...");
            generate_rose_diagram(&vgram_bins, &anisotropy, html_path)
                .map_err(|e| ToolError::Execution(format!("Failed to generate rose diagram: {}", e)))?;
        }

        let mut outputs = BTreeMap::new();
        outputs.insert(
            "directional_variogram_report".to_string(),
            json!({
                "n_directions": directions.len(),
                "n_samples": samples.len(),
                "anisotropy_ratio": anisotropy.ratio,
                "major_azimuth": anisotropy.major_azimuth,
                "output_file": output_path,
                "rose_diagram": output_html.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "not generated".to_string()),
            }),
        );

        ctx.progress.progress(1.0);
        ctx.progress.info("Directional variogram analysis complete");

        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}

/// Generate an HTML rose diagram visualization showing directional variogram ranges
fn generate_rose_diagram(
    vgram_bins: &[DirectionalVariogramBin],
    anisotropy: &AnisotropyModel,
    output_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::rendering::RadialLineGraph;

    // Extract azimuths and max semivariances (ranges) for each direction
    let azimuths: Vec<f64> = vgram_bins.iter().map(|v| v.direction_azimuth).collect();
    let ranges: Vec<f64> = vgram_bins.iter().map(|v| v.max_semivariance()).collect();
    
    // Normalize ranges to 0-1 scale for visual effect
    let max_range = ranges.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_range = ranges.iter().cloned().fold(f64::INFINITY, f64::min);
    let range_span = if (max_range - min_range).abs() > 1e-10 {
        max_range - min_range
    } else {
        1.0
    };

    let normalized_ranges: Vec<f64> = ranges
        .iter()
        .map(|r| if range_span > 0.0 { (r - min_range) / range_span } else { 0.5 })
        .collect();

    // Create RadialLineGraph with single series (rose diagram)
    let data_x = vec![azimuths];
    let data_y = vec![normalized_ranges];
    
    let graph = RadialLineGraph {
        parent_id: "rose_diagram".to_string(),
        width: 700.0,
        height: 600.0,
        data_x,
        data_y,
        series_labels: vec!["Variogram Range".to_string()],
        x_axis_label: "Azimuth".to_string(),
        x_symbol: "&theta;".to_string(),
        y_axis_label: "Normalized Range".to_string(),
        y_symbol: "&gamma;".to_string(),
        draw_points: true,
        draw_gridlines: true,
        draw_legend: true,
        draw_grey_background: false,
        fill_polygons: true,
    };

    let _filename = output_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("rose_diagram");
    
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Directional Variogram - Rose Diagram</title>{}</head><body><h1>Directional Variogram Rose Diagram</h1><p><strong>Anisotropy Analysis</strong>: Major azimuth = {:.1}°, Ratio = {:.3}<br><strong>Interpretation</strong>: {}</p><div id='rose_diagram' align=\"center\">{}</div><table align=\"center\"><tr><th>Azimuth</th><th>Range (unnormalized)</th></tr>",
        crate::rendering::html::get_css(),
        anisotropy.major_azimuth,
        anisotropy.ratio,
        if anisotropy.is_anisotropic(0.95) {
            "Strong directional dependence detected. Use anisotropic kriging."
        } else {
            "Weak or no directional dependence. Isotropic kriging is appropriate."
        },
        graph.get_svg(),
    );

    let mut table = html.to_string();
    for (azimuth, range) in vgram_bins.iter().map(|v| v.direction_azimuth).zip(ranges.iter()) {
        table.push_str(&format!(
            "<tr><td>{:.1}°</td><td class=\"numberCell\">{:.4}</td></tr>",
            azimuth, range
        ));
    }
    table.push_str("</table></body></html>");

    std::fs::write(output_path, table)?;
    Ok(())
}
