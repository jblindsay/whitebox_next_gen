//! Phase A spatial autocorrelation tools with raster output
//!
//! Implements Local Moran's I (LISA) and Local Getis-Ord G* (local hotspot analysis)
//! as raster interpolation tools. Both produce classification rasters (cluster types)
//! with configurable grid resolution and CRS handling.
//!
//! Output format is determined by file extension (.tif → GeoTIFF, .img → HFA, etc.)
//! via wbraster's automatic format detection.

use super::*;
use wbspatialstats::autocorrelation::{
    local_morans_i_lisa, LocalAssociationResult, local_getis_ord_g_star, LocalGetisOrdResult
};
use wbspatialstats::weights::{SpatialWeightsGraph, SpatialWeightsMode};

// Tool marker structs
pub struct LocalMoransILisaRasterTool;
pub struct LocalGetisOrdGStarRasterTool;

// ============================================================================
// LOCAL MORAN'S I LISA RASTER TOOL
// ============================================================================

impl Tool for LocalMoransILisaRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "local_morans_i_lisa_raster",
            display_name: "Local Moran's I LISA (Raster)",
            summary: "Computes Local Indicators of Spatial Association (LISA) as a raster grid. \
                     Classifies grid cells into cluster types: HH (hot-hot), LL (cold-cold), HL, LH, or insignificant.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "geometry", description: "Input point or polygon vector layer.", required: true },
                ToolParamSpec { name: "value_field", description: "Numeric attribute field for analysis.", required: false },
                ToolParamSpec { name: "num_neighbors", description: "Number of nearest neighbors (default 5).", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size in map units (required if no base_raster).", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("geometry".to_string(), json!("points.geojson"));
        defaults.insert("value_field".to_string(), json!("value"));
        defaults.insert("num_neighbors".to_string(), json!(5));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("lisa_classification.tif"));
        ToolManifest {
            id: "local_morans_i_lisa_raster".to_string(),
            display_name: "Local Moran's I LISA (Raster)".to_string(),
            summary: "Computes Local Indicators of Spatial Association as a classification raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "geometry".to_string(), description: "Input point or polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Numeric attribute field for analysis.".to_string(), required: false },
                ToolParamDescriptor { name: "num_neighbors".to_string(), description: "Number of nearest neighbors (default 5).".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size in map units (required if no base_raster).".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "lisa_raster_basic".to_string(),
                description: "Interpolates LISA cluster types to a raster grid.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "spatial-stats".to_string(), "autocorrelation".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "geometry")?;
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
        ctx.progress.info("Running Local Moran's I LISA raster interpolation");

        let geometry = load_vector_arg(args, "geometry")?;
        let value_field = args.get("value_field").and_then(|v| v.as_str()).unwrap_or("value");
        let num_neighbors = args.get("num_neighbors").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("Extracting point samples");
        let samples = collect_point_samples(&geometry, Some(value_field), false)?;

        if samples.len() < 3 {
            return Err(ToolError::Validation(
                "need at least 3 sample points for LISA analysis".to_string(),
            ));
        }

        ctx.progress.info("Computing Local Moran's I LISA indices");
        let coords: Vec<(f64, f64)> = samples.iter().map(|(x, y, _)| (*x, *y)).collect();
        let values: Vec<f64> = samples.iter().map(|(_, _, v)| *v).collect();

        // Build spatial weights using k-nearest neighbors
        let weights = SpatialWeightsGraph::knn_weights(&coords, num_neighbors, true)
            .map_err(|e| ToolError::Execution(format!("failed building spatial weights: {}", e)))?;

        // Compute local LISA
        let lisa_result = local_morans_i_lisa(&values, &weights)
            .map_err(|e| ToolError::Execution(format!("LISA computation failed: {}", e)))?;

        ctx.progress.progress(0.3);

        // Build output raster grid
        ctx.progress.info("Building output raster grid");
        let mut output = build_point_interpolation_output(&samples, cell_size, base_raster.as_ref())?;
        let output_crs = vector_crs_to_raster_crs(&geometry)?;
        output.configs.coordinate_ref_system = output_crs;

        let rows = output.configs.rows;
        let cols = output.configs.columns;
        let x_min = output.configs.west;
        let y_max = output.configs.north;
        let cell_x = output.configs.resolution_x.abs();
        let cell_y = output.configs.resolution_y.abs();

        // Interpolate LISA classifications to grid
        ctx.progress.info("Interpolating LISA classifications");
        let mut coalescer = PercentCoalescer::new();

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                // Find nearest sample point
                let nearest_idx = coords
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        let d_a = (a.1.0 - x).powi(2) + (a.1.1 - y).powi(2);
                        let d_b = (b.1.0 - x).powi(2) + (b.1.1 - y).powi(2);
                        d_a.partial_cmp(&d_b).unwrap()
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                // Classify LISA cluster type: 0=insignificant, 1=HH, 2=LL, 3=HL, 4=LH
                let class_value = match lisa_result.cluster_types[nearest_idx].as_str() {
                    "HH" => 1.0,
                    "LL" => 2.0,
                    "HL" => 3.0,
                    "LH" => 4.0,
                    _ => 0.0,
                };

                let idx = row * cols + col;
                output.data.set_f64(idx, class_value);
            }
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        // Output format determined by file extension
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

// ============================================================================
// LOCAL GETIS-ORD G* RASTER TOOL
// ============================================================================

impl Tool for LocalGetisOrdGStarRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "local_getis_ord_g_star_raster",
            display_name: "Local Getis-Ord G* (Raster)",
            summary: "Computes local Getis-Ord G* statistic as a raster grid. \
                     Classifies grid cells into hot spots (1), cold spots (-1), or insignificant (0).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "geometry", description: "Input point or polygon vector layer.", required: true },
                ToolParamSpec { name: "value_field", description: "Numeric attribute field for analysis.", required: false },
                ToolParamSpec { name: "num_neighbors", description: "Number of nearest neighbors (default 5).", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size in map units (required if no base_raster).", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("geometry".to_string(), json!("points.geojson"));
        defaults.insert("value_field".to_string(), json!("value"));
        defaults.insert("num_neighbors".to_string(), json!(5));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("g_star_hotspots.tif"));
        ToolManifest {
            id: "local_getis_ord_g_star_raster".to_string(),
            display_name: "Local Getis-Ord G* (Raster)".to_string(),
            summary: "Computes local Getis-Ord G* hot/cold spot statistic as a classification raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "geometry".to_string(), description: "Input point or polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "value_field".to_string(), description: "Numeric attribute field for analysis.".to_string(), required: false },
                ToolParamDescriptor { name: "num_neighbors".to_string(), description: "Number of nearest neighbors (default 5).".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size in map units (required if no base_raster).".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "g_star_raster_basic".to_string(),
                description: "Interpolates Getis-Ord G* hot/cold spots to a raster grid.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "spatial-stats".to_string(), "hotspot-analysis".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "geometry")?;
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
        ctx.progress.info("Running Local Getis-Ord G* raster interpolation");

        let geometry = load_vector_arg(args, "geometry")?;
        let value_field = args.get("value_field").and_then(|v| v.as_str()).unwrap_or("value");
        let num_neighbors = args.get("num_neighbors").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("Extracting point samples");
        let samples = collect_point_samples(&geometry, Some(value_field), false)?;

        if samples.len() < 3 {
            return Err(ToolError::Validation(
                "need at least 3 sample points for G* analysis".to_string(),
            ));
        }

        ctx.progress.info("Computing Local Getis-Ord G* indices");
        let coords: Vec<(f64, f64)> = samples.iter().map(|(x, y, _)| (*x, *y)).collect();
        let values: Vec<f64> = samples.iter().map(|(_, _, v)| *v).collect();

        // Build spatial weights using k-nearest neighbors
        let weights = SpatialWeightsGraph::knn_weights(&coords, num_neighbors, true)
            .map_err(|e| ToolError::Execution(format!("failed building spatial weights: {}", e)))?;

        // Compute local G*
        let gstar_result = local_getis_ord_g_star(&values, &weights)
            .map_err(|e| ToolError::Execution(format!("G* computation failed: {}", e)))?;

        ctx.progress.progress(0.3);

        // Build output raster grid
        ctx.progress.info("Building output raster grid");
        let mut output = build_point_interpolation_output(&samples, cell_size, base_raster.as_ref())?;
        let output_crs = vector_crs_to_raster_crs(&geometry)?;
        output.configs.coordinate_ref_system = output_crs;

        let rows = output.configs.rows;
        let cols = output.configs.columns;
        let x_min = output.configs.west;
        let y_max = output.configs.north;
        let cell_x = output.configs.resolution_x.abs();
        let cell_y = output.configs.resolution_y.abs();

        // Interpolate G* classifications to grid
        ctx.progress.info("Interpolating G* hot/cold spot classifications");
        let mut coalescer = PercentCoalescer::new();

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                // Find nearest sample point
                let nearest_idx = coords
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        let d_a = (a.1.0 - x).powi(2) + (a.1.1 - y).powi(2);
                        let d_b = (b.1.0 - x).powi(2) + (b.1.1 - y).powi(2);
                        d_a.partial_cmp(&d_b).unwrap()
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                // Classify G* hot/cold spot type: -1=cold spot, 0=insignificant, 1=hot spot
                let class_value = match gstar_result.cluster_types[nearest_idx].as_str() {
                    "HotSpot" => 1.0,
                    "ColdSpot" => -1.0,
                    _ => 0.0,
                };

                let idx = row * cols + col;
                output.data.set_f64(idx, class_value);
            }
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        // Output format determined by file extension
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

// ============================================================================
// SHARED HELPER FUNCTIONS
// ============================================================================

fn collect_point_samples(
    layer: &wbvector::Layer,
    value_field_name: Option<&str>,
    deduplicate: bool,
) -> Result<Vec<(f64, f64, f64)>, ToolError> {
    let field_name = value_field_name.unwrap_or("value");
    let field_idx = layer
        .schema
        .field_index(field_name)
        .ok_or_else(|| ToolError::Validation(format!("field '{}' not found", field_name)))?;

    let mut samples = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for feature in &layer.features {
        if let Some(geom) = &feature.geometry {
            let coords = collect_geometry_coords(geom);
            let value = feature
                .attributes
                .get(field_idx)
                .and_then(|v| match v {
                    wbvector::FieldValue::Integer(i) => Some(*i as f64),
                    wbvector::FieldValue::Float(f) => Some(*f),
                    _ => None,
                })
                .ok_or_else(|| ToolError::Execution(format!("invalid value in field '{}'", field_name)))?;

            if value.is_nan() {
                continue;
            }

            for (x, y) in coords {
                if deduplicate {
                    let key = (x.to_bits(), y.to_bits());
                    if seen.insert(key) {
                        samples.push((x, y, value));
                    }
                } else {
                    samples.push((x, y, value));
                }
            }
        }
    }

    if samples.is_empty() {
        return Err(ToolError::Execution("no valid samples found".to_string()));
    }

    Ok(samples)
}

fn collect_geometry_coords(geom: &wbvector::Geometry) -> Vec<(f64, f64)> {
    match geom {
        wbvector::Geometry::Point { x, y } => vec![(*x, *y)],
        wbvector::Geometry::LineString { points } => points.clone(),
        wbvector::Geometry::Polygon { exterior, .. } => exterior.clone(),
        wbvector::Geometry::MultiPoint { points } => {
            points.iter().map(|(x, y)| (*x, *y)).collect()
        }
        wbvector::Geometry::MultiLineString { lines } => {
            lines.iter().flat_map(|line| line.clone()).collect()
        }
        wbvector::Geometry::MultiPolygon { polygons } => {
            polygons
                .iter()
                .flat_map(|(exterior, _)| exterior.clone())
                .collect()
        }
    }
}
