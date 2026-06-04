use super::*;
use wbspatialstats::weights;
use wbspatialstats::autocorrelation;

// Re-export from wbspatialstats for convenience
use weights::SpatialWeightsMode;
use weights::IslandPolicy;

// Helper trait to convert wbspatialstats enums to/from strings for Tool args
trait WeightsModeExt {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError>
    where
        Self: Sized;
}

impl WeightsModeExt for SpatialWeightsMode {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError> {
        let text = args
            .get("weights_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("k_nearest")
            .trim()
            .to_ascii_lowercase();
        SpatialWeightsMode::from_str(&text).ok_or_else(|| {
            ToolError::Validation(
                "weights_mode must be one of: queen, rook, k_nearest, distance_band".to_string(),
            )
        })
    }
}

trait IslandPolicyExt {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError>
    where
        Self: Sized;
}

impl IslandPolicyExt for IslandPolicy {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError> {
        let text = args
            .get("island_policy")
            .and_then(|v| v.as_str())
            .unwrap_or("drop_with_warning")
            .trim()
            .to_ascii_lowercase();
        IslandPolicy::from_str(&text).ok_or_else(|| {
            ToolError::Validation(
                "island_policy must be one of: drop_with_warning, keep_zero_weight, error".to_string(),
            )
        })
    }
}

#[derive(Clone)]
struct SpatialObservation {
    source_index: usize,
    x: f64,
    y: f64,
    value: f64,
    topo: Option<TopoGeometry>,
}

pub struct GlobalMoransITool;
pub struct LocalMoransILisaTool;
pub struct LocalMoransILisaRasterTool;
pub struct GetisOrdGiStarTool;
pub struct GetisOrdGiStarRasterTool;
pub struct NearestNeighbourIndexTool;
pub struct QuadratCountTestTool;
pub struct SpatialLagRegressionTool;
pub struct SpatialLagRegressionRasterTool;
pub struct SpatialErrorRegressionTool;
pub struct SpatialErrorRegressionRasterTool;
pub struct GeographicallyWeightedRegressionTool;
pub struct GeographicallyWeightedRegressionRasterTool;

fn parse_optional_usize_arg(args: &ToolArgs, key: &str) -> Result<Option<usize>, ToolError> {
    match args.get(key) {
        None => Ok(None),
        Some(value) => {
            let Some(raw) = value.as_i64() else {
                return Err(ToolError::Validation(format!("parameter '{}' must be an integer", key)));
            };
            if raw <= 0 {
                return Err(ToolError::Validation(format!("parameter '{}' must be > 0", key)));
            }
            Ok(Some(raw as usize))
        }
    }
}

fn normal_cdf(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * z);
    let poly = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let cdf = 1.0 - pdf * poly;
    if x >= 0.0 { cdf } else { 1.0 - cdf }
}

fn two_tailed_normal_p(z: f64) -> f64 {
    (2.0 * (1.0 - normal_cdf(z.abs()))).clamp(0.0, 1.0)
}

fn collect_spatial_observations(layer: &wbvector::Layer, field: &str) -> Result<(Vec<SpatialObservation>, usize), ToolError> {
    let field_idx = layer
        .schema
        .field_index(field)
        .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", field)))?;

    let mut observations = Vec::<SpatialObservation>::new();
    let mut dropped = 0usize;

    for (source_index, feature) in layer.features.iter().enumerate() {
        let Some(geometry) = &feature.geometry else {
            dropped += 1;
            continue;
        };
        let Some(value) = feature.attributes.get(field_idx).and_then(|v| v.as_f64()) else {
            dropped += 1;
            continue;
        };
        if !value.is_finite() {
            dropped += 1;
            continue;
        }

        let centroid = match geometry {
            wbvector::Geometry::Point(coord) => Some((coord.x, coord.y)),
            _ => {
                let topo = super::wb_geometry_to_topo(geometry)?;
                geometry_centroid(&topo).map(|c| (c.x, c.y))
            }
        };
        let Some((x, y)) = centroid else {
            dropped += 1;
            continue;
        };

        observations.push(SpatialObservation {
            source_index,
            x,
            y,
            value,
            topo: Some(super::wb_geometry_to_topo(geometry)?),
        });
    }

    if observations.len() < 3 {
        return Err(ToolError::Validation(
            "global_morans_i requires at least 3 valid features after filtering".to_string(),
        ));
    }

    Ok((observations, dropped))
}

fn build_distance_neighbors(
    observations: &[SpatialObservation],
    mode: SpatialWeightsMode,
    k: usize,
    distance_band: f64,
) -> Result<Vec<Vec<(usize, f64)>>, ToolError> {
    let mut tree = KdTree::new(2);
    for (idx, obs) in observations.iter().enumerate() {
        tree.add([obs.x, obs.y], idx)
            .map_err(|e| ToolError::Execution(format!("failed building k-d tree: {e}")))?;
    }

    let mut neighbors = vec![Vec::<(usize, f64)>::new(); observations.len()];
    for (i, obs) in observations.iter().enumerate() {
        let query = [obs.x, obs.y];
        let entries = match mode {
            SpatialWeightsMode::KNearest => tree
                .nearest(&query, k.saturating_add(1), &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("k-nearest query failed: {e}")))?,
            SpatialWeightsMode::DistanceBand => tree
                .within(&query, distance_band * distance_band, &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("distance-band query failed: {e}")))?,
            _ => Vec::new(),
        };

        for (dist_sq, jref) in entries {
            let j = *jref;
            if i == j {
                continue;
            }
            let dist = dist_sq.sqrt();
            if matches!(mode, SpatialWeightsMode::DistanceBand) && dist > distance_band {
                continue;
            }
            if dist == 0.0 {
                continue;
            }
            neighbors[i].push((j, 1.0 / dist.max(1.0e-12)));
        }
    }

    Ok(neighbors)
}

fn build_contiguity_neighbors(
    observations: &[SpatialObservation],
    mode: SpatialWeightsMode,
) -> Result<(Vec<Vec<(usize, f64)>>, bool), ToolError> {
    let geoms: Vec<TopoGeometry> = observations
        .iter()
        .map(|obs| {
            obs.topo.clone().ok_or_else(|| {
                ToolError::Validation(
                    "contiguity weights require valid feature geometries".to_string(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let index = SpatialIndex::from_geometries(&geoms);
    let mut neighbors = vec![Vec::<(usize, f64)>::new(); geoms.len()];

    // Rook approximation note: this current bounded implementation uses the same
    // topology predicate pathway as queen while preserving deterministic behavior.
    let rook_approximation = matches!(mode, SpatialWeightsMode::Rook);

    for i in 0..geoms.len() {
        let candidates = index.query_geometry(&geoms[i]);
        for j in candidates {
            if i == j {
                continue;
            }
            let linked = intersects(&geoms[i], &geoms[j]);
            if linked {
                neighbors[i].push((j, 1.0));
            }
        }
    }

    Ok((neighbors, rook_approximation))
}

fn build_spatial_weights(
    observations: &[SpatialObservation],
    mode: SpatialWeightsMode,
    row_standardize: bool,
    island_policy: IslandPolicy,
    k: usize,
    distance_band: f64,
    dropped_feature_count: usize,
) -> Result<weights::SpatialWeightsGraph, ToolError> {
    let (mut neighbors, rook_approximation) = match mode {
        SpatialWeightsMode::Queen | SpatialWeightsMode::Rook => {
            let (n, approx) = build_contiguity_neighbors(observations, mode)?;
            (n, approx)
        }
        SpatialWeightsMode::KNearest | SpatialWeightsMode::DistanceBand => (
            build_distance_neighbors(observations, mode, k, distance_band)?,
            false,
        ),
    };

    for row in &mut neighbors {
        row.sort_by_key(|(idx, _)| *idx);
        row.dedup_by_key(|(idx, _)| *idx);
    }

    let mut warnings = Vec::<String>::new();
    let island_count = neighbors.iter().filter(|n| n.is_empty()).count();
    if island_count > 0 {
        match island_policy {
            IslandPolicy::DropWithWarning => {
                warnings.push(format!(
                    "{} features had zero neighbours and were dropped from analysis",
                    island_count
                ));
            }
            IslandPolicy::KeepZeroWeight => {
                warnings.push(format!(
                    "{} features had zero neighbours and were retained with zero-weight rows",
                    island_count
                ));
            }
            IslandPolicy::Error => {
                return Err(ToolError::Validation(format!(
                    "{} features have zero neighbours under the selected weights configuration",
                    island_count
                )));
            }
        }
    }

    if rook_approximation {
        warnings.push(
            "rook contiguity currently uses a bounded queen-like topology predicate approximation"
                .to_string(),
        );
    }

    if row_standardize {
        for row in &mut neighbors {
            let row_sum: f64 = row.iter().map(|(_, w)| *w).sum();
            if row_sum > 0.0 {
                for (_, w) in row {
                    *w /= row_sum;
                }
            }
        }
    }

    let counts: Vec<usize> = neighbors.iter().map(|n| n.len()).collect();
    let min_neighbors = *counts.iter().min().unwrap_or(&0);
    let max_neighbors = *counts.iter().max().unwrap_or(&0);
    let mean_neighbors = if counts.is_empty() {
        0.0
    } else {
        counts.iter().sum::<usize>() as f64 / counts.len() as f64
    };

    let diagnostics = weights::SpatialWeightsDiagnostics {
        n_features: observations.len(),
        n_islands: island_count,
        neighbor_count_min: min_neighbors,
        neighbor_count_mean: mean_neighbors,
        neighbor_count_max: max_neighbors,
        connected_component_count: weights::connected_components(&neighbors),
        row_standardized: row_standardize,
        dropped_feature_count,
    };

    Ok(weights::SpatialWeightsGraph {
        neighbors,
        diagnostics,
        warnings,
    })
}

fn compute_global_morans_i(
    values: &[f64],
    raw_weights: &weights::SpatialWeightsGraph,
    island_policy: IslandPolicy,
) -> Result<(f64, f64, f64, f64, usize), ToolError> {
    // Filter for islands if needed
    let n_total = values.len();
    let mut included = vec![true; n_total];
    if matches!(island_policy, IslandPolicy::DropWithWarning) {
        for (i, row) in raw_weights.neighbors.iter().enumerate() {
            if row.is_empty() {
                included[i] = false;
            }
        }
    }

    let idxs: Vec<usize> = included
        .iter()
        .enumerate()
        .filter_map(|(i, keep)| if *keep { Some(i) } else { None })
        .collect();
    
    if idxs.len() < 3 {
        return Err(ToolError::Validation(
            "insufficient connected observations after island handling".to_string(),
        ));
    }

    // Build filtered weights and values
    let mut filtered_values = Vec::new();
    let mut index_map = vec![None; n_total];
    for (new_idx, &old_idx) in idxs.iter().enumerate() {
        index_map[old_idx] = Some(new_idx);
        filtered_values.push(values[old_idx]);
    }

    let mut filtered_neighbors: Vec<Vec<(usize, f64)>> = vec![Vec::new(); idxs.len()];
    for (new_i, &old_i) in idxs.iter().enumerate() {
        for (old_j, weight) in &raw_weights.neighbors[old_i] {
            if let Some(new_j) = index_map[*old_j] {
                filtered_neighbors[new_i].push((new_j, *weight));
            }
        }
    }

    let filtered_weights = weights::SpatialWeightsGraph {
        neighbors: filtered_neighbors,
        diagnostics: raw_weights.diagnostics.clone(),
        warnings: vec![],
    };

    // Call wbspatialstats function
    let result = autocorrelation::morans_i(&filtered_values, &filtered_weights)
        .map_err(|e| ToolError::Validation(format!("Moran's I computation failed: {}", e)))?;

    Ok((
        result.statistic,
        result.expected_value,
        result.z_score,
        result.p_value,
        idxs.len(),
    ))
}

/// Wrapper that handles island filtering and calls wbspatialstats::autocorrelation::local_morans_i_lisa()
fn compute_local_morans_i_lisa(
    values: &[f64],
    raw_weights: &weights::SpatialWeightsGraph,
    island_policy: IslandPolicy,
    alpha: f64,
) -> Result<(Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>, Vec<String>), ToolError> {
    let n_total = values.len();
    let mut included = vec![true; n_total];
    if matches!(island_policy, IslandPolicy::DropWithWarning) {
        for (i, row) in raw_weights.neighbors.iter().enumerate() {
            if row.is_empty() {
                included[i] = false;
            }
        }
    }

    let idxs: Vec<usize> = included
        .iter()
        .enumerate()
        .filter_map(|(i, keep)| if *keep { Some(i) } else { None })
        .collect();

    if idxs.len() < 3 {
        return Err(ToolError::Validation(
            "insufficient connected observations after island handling".to_string(),
        ));
    }

    // Build filtered weights and values
    let mut filtered_values = Vec::new();
    let mut index_map = vec![None; n_total];
    for (new_idx, &old_idx) in idxs.iter().enumerate() {
        index_map[old_idx] = Some(new_idx);
        filtered_values.push(values[old_idx]);
    }

    let mut filtered_neighbors: Vec<Vec<(usize, f64)>> = vec![Vec::new(); idxs.len()];
    for (new_i, &old_i) in idxs.iter().enumerate() {
        for (old_j, weight) in &raw_weights.neighbors[old_i] {
            if let Some(new_j) = index_map[*old_j] {
                filtered_neighbors[new_i].push((new_j, *weight));
            }
        }
    }

    let filtered_weights = weights::SpatialWeightsGraph {
        neighbors: filtered_neighbors,
        diagnostics: raw_weights.diagnostics.clone(),
        warnings: vec![],
    };

    // Call wbspatialstats function
    let result = autocorrelation::local_morans_i_lisa(&filtered_values, &filtered_weights, alpha)
        .map_err(|e| ToolError::Validation(format!("LISA computation failed: {}", e)))?;

    // Map results back to original indices
    let mut lisa_i = vec![None; n_total];
    let mut lisa_z = vec![None; n_total];
    let mut lisa_p = vec![None; n_total];
    let mut quadrant = vec!["NS".to_string(); n_total];

    for (new_i, &old_i) in idxs.iter().enumerate() {
        lisa_i[old_i] = Some(result.local_statistics[new_i]);
        lisa_z[old_i] = Some(result.z_scores[new_i]);
        lisa_p[old_i] = Some(result.p_values[new_i]);
        quadrant[old_i] = match result.cluster_types[new_i].as_str() {
            "HH" => "HH",
            "LL" => "LL",
            "HL" => "HL",
            "LH" => "LH",
            _ => "NS",
        }
        .to_string();
    }

    Ok((lisa_i, lisa_z, lisa_p, quadrant))
}

/// Wrapper that handles island filtering and calls wbspatialstats::autocorrelation::getis_ord_g_star()
fn compute_getis_ord_gi_star(
    values: &[f64],
    raw_weights: &weights::SpatialWeightsGraph,
    island_policy: IslandPolicy,
    alpha: f64,
) -> Result<(Vec<Option<f64>>, Vec<Option<f64>>, Vec<String>), ToolError> {
    let n_total = values.len();
    let mut included = vec![true; n_total];
    if matches!(island_policy, IslandPolicy::DropWithWarning) {
        for (i, row) in raw_weights.neighbors.iter().enumerate() {
            if row.is_empty() {
                included[i] = false;
            }
        }
    }

    let idxs: Vec<usize> = included
        .iter()
        .enumerate()
        .filter_map(|(i, keep)| if *keep { Some(i) } else { None })
        .collect();

    if idxs.len() < 3 {
        return Err(ToolError::Validation(
            "insufficient connected observations after island handling".to_string(),
        ));
    }

    // Build filtered weights and values
    let mut filtered_values = Vec::new();
    let mut index_map = vec![None; n_total];
    for (new_idx, &old_idx) in idxs.iter().enumerate() {
        index_map[old_idx] = Some(new_idx);
        filtered_values.push(values[old_idx]);
    }

    let mut filtered_neighbors: Vec<Vec<(usize, f64)>> = vec![Vec::new(); idxs.len()];
    for (new_i, &old_i) in idxs.iter().enumerate() {
        for (old_j, weight) in &raw_weights.neighbors[old_i] {
            if let Some(new_j) = index_map[*old_j] {
                filtered_neighbors[new_i].push((new_j, *weight));
            }
        }
    }

    let filtered_weights = weights::SpatialWeightsGraph {
        neighbors: filtered_neighbors,
        diagnostics: raw_weights.diagnostics.clone(),
        warnings: vec![],
    };

    // Call wbspatialstats function
    let result = autocorrelation::getis_ord_g_star(&filtered_values, &filtered_weights, alpha)
        .map_err(|e| ToolError::Validation(format!("Getis-Ord G* computation failed: {}", e)))?;

    // Map results back to original indices
    let mut gi_z = vec![None; n_total];
    let mut gi_p = vec![None; n_total];
    let mut cluster_type = vec!["insignificant".to_string(); n_total];

    for (new_i, &old_i) in idxs.iter().enumerate() {
        gi_z[old_i] = Some(result.z_scores[new_i]);
        gi_p[old_i] = Some(result.p_values[new_i]);
        cluster_type[old_i] = match result.cluster_types[new_i].as_str() {
            "HotSpot" => "HotSpot",
            "ColdSpot" => "ColdSpot",
            _ => "insignificant",
        }
        .to_string();
    }

    Ok((gi_z, gi_p, cluster_type))
}

fn write_text(path: &std::path::Path, contents: &str) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
        }
    }
    std::fs::write(path, contents)
        .map_err(|e| ToolError::Execution(format!("failed writing report output: {e}")))
}

fn build_branded_html_report(title: &str, headers: &[&str], row_values: &[String]) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">\n");
    html.push_str("<html xmlns=\"http://www.w3.org/1999/xhtml\"><head><meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\" />\n");
    html.push_str(&format!("<title>{}</title>\n", title));
    html.push_str(&crate::rendering::html::get_css());
    html.push_str("</head><body>\n");
    html.push_str(&format!("<h1>{}</h1>\n", title));
    html.push_str("<div><table align=\"center\">\n<tr>");
    for header in headers {
        html.push_str(&format!("<th>{}</th>", header));
    }
    html.push_str("</tr>\n<tr>");
    for value in row_values {
        html.push_str(&format!("<td class=\"numberCell\">{}</td>", value));
    }
    html.push_str("</tr>\n</table></div>\n</body></html>");
    html
}

impl Tool for GlobalMoransITool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "global_morans_i",
            display_name: "Global Moran's I",
            summary: "Computes Global Moran's I with diagnostics and asymptotic significance.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field to analyze.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode.", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "inference", description: "Inference mode: asymptotic (current) or permutation (future).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "output_json", description: "Optional JSON report output path.", required: false },
                ToolParamSpec { name: "output_html", description: "Optional HTML report output path.", required: false },
                ToolParamSpec { name: "output_csv", description: "Optional CSV summary output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("weights_mode".to_string(), json!("k_nearest"));
        defaults.insert("k".to_string(), json!(8));
        defaults.insert("row_standardize".to_string(), json!(true));
        defaults.insert("inference".to_string(), json!("asymptotic"));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));

        let mut example_args = defaults.clone();
        example_args.insert("output_json".to_string(), json!("morans_i_report.json"));

        ToolManifest {
            id: "global_morans_i".to_string(),
            display_name: "Global Moran's I".to_string(),
            summary: "Computes Global Moran's I with diagnostics and asymptotic significance.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field to analyze.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode: queen, rook, k_nearest, distance_band.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest mode.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold for distance_band mode.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Apply row standardization to weights (default true).".to_string(), required: false },
                ToolParamDescriptor { name: "inference".to_string(), description: "Inference mode: asymptotic (current) or permutation (future).".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "output_json".to_string(), description: "Optional JSON report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_csv".to_string(), description: "Optional CSV summary output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "global_morans_i_basic".to_string(),
                description: "Computes Global Moran's I and writes a JSON summary report.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "spatial-statistics".to_string(),
                "autocorrelation".to_string(),
                "report".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        if field.trim().is_empty() {
            return Err(ToolError::Validation("field must be non-empty".to_string()));
        }

        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        if matches!(mode, SpatialWeightsMode::KNearest) && k == 0 {
            return Err(ToolError::Validation("k must be > 0".to_string()));
        }

        if matches!(mode, SpatialWeightsMode::DistanceBand) {
            let d = parse_f64_arg(args, "distance")?;
            if !d.is_finite() || d <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }

        if let Some(distance) = parse_optional_f64_arg(args, "distance") {
            if !distance.is_finite() || distance <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }

        let inference = args
            .get("inference")
            .and_then(|v| v.as_str())
            .unwrap_or("asymptotic")
            .trim()
            .to_ascii_lowercase();
        if inference != "asymptotic" && inference != "permutation" {
            return Err(ToolError::Validation(
                "inference must be one of: asymptotic, permutation".to_string(),
            ));
        }

        let _ = IslandPolicy::parse(args)?;
        let _ = parse_optional_output_path(args, "output_json")?;
        let _ = parse_optional_output_path(args, "output_html")?;
        let _ = parse_optional_output_path(args, "output_csv")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let inference = args
            .get("inference")
            .and_then(|v| v.as_str())
            .unwrap_or("asymptotic")
            .trim()
            .to_ascii_lowercase();
        let island_policy = IslandPolicy::parse(args)?;

        let output_json = parse_optional_output_path(args, "output_json")?;
        let output_html = parse_optional_output_path(args, "output_html")?;
        let output_csv = parse_optional_output_path(args, "output_csv")?;

        if inference == "permutation" {
            return Err(ToolError::Validation(
                "permutation inference is not implemented yet for global_morans_i; use inference='asymptotic'"
                    .to_string(),
            ));
        }

        let (observations, dropped) = collect_spatial_observations(&input, &field)?;
        let values: Vec<f64> = observations.iter().map(|o| o.value).collect();

        ctx.progress.info("building spatial weights");
        let weights = build_spatial_weights(
            &observations,
            mode,
            row_standardize,
            island_policy,
            k,
            distance,
            dropped,
        )?;

        ctx.progress.info("computing Moran's I");
        let (statistic_i, expected_i, z_score, p_value, n_used) =
            compute_global_morans_i(&values, &weights, island_policy)?;

        let mut report = serde_json::Map::new();
        report.insert("tool_id".to_string(), json!("global_morans_i"));
        report.insert("inference_method".to_string(), json!("asymptotic"));
        report.insert("statistic_i".to_string(), json!(statistic_i));
        report.insert("expected_i".to_string(), json!(expected_i));
        report.insert(
            "variance_i".to_string(),
            json!(((statistic_i - expected_i) / z_score).powi(2)),
        );
        report.insert("z_score".to_string(), json!(z_score));
        report.insert("p_value_two_sided".to_string(), json!(p_value));
        report.insert("n_features_used".to_string(), json!(n_used));
        report.insert("n_features_dropped".to_string(), json!(weights.diagnostics.dropped_feature_count));
        report.insert(
            "weights_diagnostics".to_string(),
            json!({
                "n_features": weights.diagnostics.n_features,
                "n_islands": weights.diagnostics.n_islands,
                "neighbor_count_min": weights.diagnostics.neighbor_count_min,
                "neighbor_count_mean": weights.diagnostics.neighbor_count_mean,
                "neighbor_count_max": weights.diagnostics.neighbor_count_max,
                "connected_component_count": weights.diagnostics.connected_component_count,
                "row_standardized": weights.diagnostics.row_standardized,
            }),
        );
        report.insert(
            "warnings".to_string(),
            json!(weights.warnings),
        );
        report.insert("statistic".to_string(), json!(statistic_i));
        report.insert("p_value".to_string(), json!(p_value));
        report.insert("alpha".to_string(), serde_json::Value::Null);
        report.insert("n_observations".to_string(), json!(n_used));
        report.insert(
            "dropped_observations".to_string(),
            json!(weights.diagnostics.dropped_feature_count),
        );
        let significance_class = if p_value <= 0.05 && z_score > 0.0 {
            "positive"
        } else if p_value <= 0.05 && z_score < 0.0 {
            "negative"
        } else {
            "ns"
        };
        report.insert("significance_class".to_string(), json!(significance_class));
        report.insert(
            "assumption_flags".to_string(),
            json!({
                "permutation_supported": false,
                "inference": "asymptotic",
            }),
        );
        report.insert(
            "runtime_metadata".to_string(),
            json!({
                "seed": serde_json::Value::Null,
                "permutations": serde_json::Value::Null,
            }),
        );

        let report_value = serde_json::Value::Object(report);

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), report_value.clone());
        outputs.insert("summary".to_string(), report_value.clone());

        if let Some(path) = output_json {
            let body = serde_json::to_string_pretty(&report_value)
                .map_err(|e| ToolError::Execution(format!("failed serializing JSON report: {e}")))?;
            write_text(&path, &body)?;
            outputs.insert("output_json".to_string(), json!(path.to_string_lossy().to_string()));
        }

        if let Some(path) = output_csv {
            let z_text = z_score.to_string();
            let p_text = p_value.to_string();
            let body = format!(
                "tool_id,statistic_i,expected_i,z_score,p_value_two_sided,n_features_used,n_features_dropped\nglobal_morans_i,{},{},{},{},{},{}\n",
                statistic_i,
                expected_i,
                z_text,
                p_text,
                n_used,
                weights.diagnostics.dropped_feature_count,
            );
            write_text(&path, &body)?;
            outputs.insert("output_csv".to_string(), json!(path.to_string_lossy().to_string()));
        }

        if let Some(path) = output_html {
            let z_text = format!("{z_score:.6}");
            let p_text = format!("{p_value:.6}");
            let body = build_branded_html_report(
                "Global Moran's I Report",
                &[
                    "Statistic I",
                    "Expected I",
                    "Z",
                    "P (two-sided)",
                    "N used",
                    "N dropped",
                ],
                &[
                    format!("{statistic_i:.6}"),
                    format!("{expected_i:.6}"),
                    z_text,
                    p_text,
                    n_used.to_string(),
                    weights.diagnostics.dropped_feature_count.to_string(),
                ],
            );
            write_text(&path, &body)?;
            outputs.insert("output_html".to_string(), json!(path.to_string_lossy().to_string()));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

#[derive(Clone, Copy)]
enum MultipleTestingMode {
    None,
    FdrBh,
    Bonferroni,
}

impl MultipleTestingMode {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError> {
        let text = args
            .get("multiple_testing")
            .and_then(|v| v.as_str())
            .unwrap_or("fdr_bh")
            .trim()
            .to_ascii_lowercase();
        match text.as_str() {
            "none" => Ok(Self::None),
            "fdr_bh" => Ok(Self::FdrBh),
            "bonferroni" => Ok(Self::Bonferroni),
            _ => Err(ToolError::Validation(
                "multiple_testing must be one of: none, fdr_bh, bonferroni".to_string(),
            )),
        }
    }
}

fn adjust_p_values(raw: &[Option<f64>], mode: MultipleTestingMode) -> Vec<Option<f64>> {
    let mut adjusted = vec![None; raw.len()];
    let mut pairs: Vec<(usize, f64)> = raw
        .iter()
        .enumerate()
        .filter_map(|(idx, p)| p.map(|v| (idx, v.clamp(0.0, 1.0))))
        .collect();

    if pairs.is_empty() {
        return adjusted;
    }

    match mode {
        MultipleTestingMode::None => {
            for (idx, p) in pairs {
                adjusted[idx] = Some(p);
            }
        }
        MultipleTestingMode::Bonferroni => {
            let m = pairs.len() as f64;
            for (idx, p) in pairs {
                adjusted[idx] = Some((p * m).min(1.0));
            }
        }
        MultipleTestingMode::FdrBh => {
            pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let m = pairs.len();
            let mut ranked = vec![0.0f64; m];
            for (rank, (_, p)) in pairs.iter().enumerate() {
                ranked[rank] = (p * m as f64 / (rank as f64 + 1.0)).min(1.0);
            }
            for i in (0..m.saturating_sub(1)).rev() {
                ranked[i] = ranked[i].min(ranked[i + 1]);
            }
            for (rank, (idx, _)) in pairs.iter().enumerate() {
                adjusted[*idx] = Some(ranked[rank]);
            }
        }
    }

    adjusted
}

impl Tool for LocalMoransILisaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "local_morans_i_lisa",
            display_name: "Local Moran's I (LISA)",
            summary: "Computes Local Moran's I (LISA) and writes per-feature significance classes.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field to analyze.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode.", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "inference", description: "Inference mode: asymptotic (current) or permutation (future).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "alpha", description: "Significance threshold in [0, 1]; default 0.05.", required: false },
                ToolParamSpec { name: "multiple_testing", description: "Multiple-testing correction: none, fdr_bh, bonferroni.", required: false },
                ToolParamSpec { name: "output", description: "Output vector path with LISA fields.", required: true },
                ToolParamSpec { name: "output_html", description: "Optional HTML report output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("weights_mode".to_string(), json!("k_nearest"));
        defaults.insert("k".to_string(), json!(8));
        defaults.insert("row_standardize".to_string(), json!(true));
        defaults.insert("inference".to_string(), json!("asymptotic"));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));
        defaults.insert("alpha".to_string(), json!(0.05));
        defaults.insert("multiple_testing".to_string(), json!("fdr_bh"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("lisa_output.gpkg"));

        ToolManifest {
            id: "local_morans_i_lisa".to_string(),
            display_name: "Local Moran's I (LISA)".to_string(),
            summary: "Computes Local Moran's I (LISA) and writes per-feature significance classes.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field to analyze.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode: queen, rook, k_nearest, distance_band.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest mode.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold for distance_band mode.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Apply row standardization to weights (default true).".to_string(), required: false },
                ToolParamDescriptor { name: "inference".to_string(), description: "Inference mode: asymptotic (current) or permutation (future).".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "alpha".to_string(), description: "Significance threshold in [0, 1]; default 0.05.".to_string(), required: false },
                ToolParamDescriptor { name: "multiple_testing".to_string(), description: "Multiple-testing correction: none, fdr_bh, bonferroni.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path with LISA fields.".to_string(), required: true },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "local_morans_i_lisa_basic".to_string(),
                description: "Computes Local Moran's I and writes per-feature LISA fields.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "spatial-statistics".to_string(),
                "autocorrelation".to_string(),
                "lisa".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        if field.trim().is_empty() {
            return Err(ToolError::Validation("field must be non-empty".to_string()));
        }

        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        if matches!(mode, SpatialWeightsMode::KNearest) && k == 0 {
            return Err(ToolError::Validation("k must be > 0".to_string()));
        }
        if matches!(mode, SpatialWeightsMode::DistanceBand) {
            let d = parse_f64_arg(args, "distance")?;
            if !d.is_finite() || d <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }
        if let Some(distance) = parse_optional_f64_arg(args, "distance") {
            if !distance.is_finite() || distance <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }

        let inference = args
            .get("inference")
            .and_then(|v| v.as_str())
            .unwrap_or("asymptotic")
            .trim()
            .to_ascii_lowercase();
        if inference != "asymptotic" && inference != "permutation" {
            return Err(ToolError::Validation(
                "inference must be one of: asymptotic, permutation".to_string(),
            ));
        }

        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(ToolError::Validation("alpha must be in [0, 1]".to_string()));
        }

        let _ = IslandPolicy::parse(args)?;
        let _ = MultipleTestingMode::parse(args)?;
        let _ = parse_vector_path_arg(args, "output")?;
        let _ = parse_optional_output_path(args, "output_html")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let inference = args
            .get("inference")
            .and_then(|v| v.as_str())
            .unwrap_or("asymptotic")
            .trim()
            .to_ascii_lowercase();
        let island_policy = IslandPolicy::parse(args)?;
        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        let multiple_testing = MultipleTestingMode::parse(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let output_html = parse_optional_output_path(args, "output_html")?;

        if inference == "permutation" {
            return Err(ToolError::Validation(
                "permutation inference is not implemented yet for local_morans_i_lisa; use inference='asymptotic'"
                    .to_string(),
            ));
        }

        let (observations, dropped) = collect_spatial_observations(&input, &field)?;
        let values: Vec<f64> = observations.iter().map(|o| o.value).collect();

        ctx.progress.info("building spatial weights");
        let weights = build_spatial_weights(
            &observations,
            mode,
            row_standardize,
            island_policy,
            k,
            distance,
            dropped,
        )?;

        ctx.progress.info("computing LISA");
        let (lisa_i, lisa_z, lisa_p, quadrant) = compute_local_morans_i_lisa(&values, &weights, island_policy, alpha)?;

        // Count islands for reporting (features with no neighbors after island filtering)
        let n_obs = observations.len();
        let mut island_count = 0usize;
        if matches!(island_policy, IslandPolicy::DropWithWarning) {
            for i in 0..n_obs {
                if weights.neighbors[i].is_empty() {
                    island_count += 1;
                }
            }
        }

        let lisa_p_adj = adjust_p_values(&lisa_p, multiple_testing);

        let mut output = input.clone();
        let mut schema = output.schema.clone();
        for field_name in ["LISA_I", "LISA_Z", "LISA_P", "LISA_P_ADJ", "LISA_SIG", "LISA_CLASS"] {
            if schema.field_index(field_name).is_some() {
                return Err(ToolError::Validation(format!(
                    "output schema already contains field '{}'", field_name
                )));
            }
        }
        schema.add_field(wbvector::FieldDef::new("LISA_I", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("LISA_Z", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("LISA_P", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("LISA_P_ADJ", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("LISA_SIG", wbvector::FieldType::Integer));
        schema.add_field(wbvector::FieldDef::new("LISA_CLASS", wbvector::FieldType::Text));
        output.schema = schema;

        let mut obs_by_source = vec![None; input.features.len()];
        for (obs_idx, obs) in observations.iter().enumerate() {
            obs_by_source[obs.source_index] = Some(obs_idx);
        }

        let mut hh = 0usize;
        let mut ll = 0usize;
        let mut hl = 0usize;
        let mut lh = 0usize;
        let mut ns = 0usize;

        for feature_index in 0..output.features.len() {
            if let Some(obs_idx) = obs_by_source[feature_index] {
                let p_adj = lisa_p_adj[obs_idx];
                let sig = p_adj.is_some_and(|p| p <= alpha);
                let class = if sig {
                    quadrant[obs_idx].as_str()
                } else {
                    "NS"
                };
                match class {
                    "HH" => hh += 1,
                    "LL" => ll += 1,
                    "HL" => hl += 1,
                    "LH" => lh += 1,
                    _ => ns += 1,
                }

                output.features[feature_index]
                    .attributes
                    .push(lisa_i[obs_idx].map_or(wbvector::FieldValue::Null, wbvector::FieldValue::Float));
                output.features[feature_index]
                    .attributes
                    .push(lisa_z[obs_idx].map_or(wbvector::FieldValue::Null, wbvector::FieldValue::Float));
                output.features[feature_index]
                    .attributes
                    .push(lisa_p[obs_idx].map_or(wbvector::FieldValue::Null, wbvector::FieldValue::Float));
                output.features[feature_index]
                    .attributes
                    .push(p_adj.map_or(wbvector::FieldValue::Null, wbvector::FieldValue::Float));
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Integer(if sig { 1 } else { 0 }));
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Text(class.to_string()));
            } else {
                ns += 1;
                output.features[feature_index].attributes.push(wbvector::FieldValue::Null);
                output.features[feature_index].attributes.push(wbvector::FieldValue::Null);
                output.features[feature_index].attributes.push(wbvector::FieldValue::Null);
                output.features[feature_index].attributes.push(wbvector::FieldValue::Null);
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Integer(0));
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Text("NS".to_string()));
            }
        }

        let locator = write_vector_output(&output, output_path.trim())?;

        let n_features_used = n_obs - weights.diagnostics.dropped_feature_count - island_count;

        let summary = json!({
                "tool_id": "local_morans_i_lisa",
                "inference_method": "asymptotic",
                "statistic": serde_json::Value::Null,
                "p_value": serde_json::Value::Null,
                "alpha": alpha,
                "significance_class": serde_json::Value::Null,
                "multiple_testing": match multiple_testing {
                    MultipleTestingMode::None => "none",
                    MultipleTestingMode::FdrBh => "fdr_bh",
                    MultipleTestingMode::Bonferroni => "bonferroni",
                },
                "n_features_used": n_features_used,
                "n_features_dropped": weights.diagnostics.dropped_feature_count,
                "n_observations": n_features_used,
                "dropped_observations": weights.diagnostics.dropped_feature_count,
                "n_islands": island_count,
                "class_counts": {
                    "HH": hh,
                    "LL": ll,
                    "HL": hl,
                    "LH": lh,
                    "NS": ns,
                },
                "weights_diagnostics": {
                    "n_features": weights.diagnostics.n_features,
                    "n_islands": weights.diagnostics.n_islands,
                    "neighbor_count_min": weights.diagnostics.neighbor_count_min,
                    "neighbor_count_mean": weights.diagnostics.neighbor_count_mean,
                    "neighbor_count_max": weights.diagnostics.neighbor_count_max,
                    "connected_component_count": weights.diagnostics.connected_component_count,
                    "row_standardized": weights.diagnostics.row_standardized,
                },
                "warnings": weights.warnings,
                "assumption_flags": {
                    "permutation_supported": false,
                    "inference": "asymptotic",
                },
                "runtime_metadata": {
                    "seed": serde_json::Value::Null,
                    "permutations": serde_json::Value::Null,
                },
            });

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), json!(locator));
        outputs.insert("summary".to_string(), summary.clone());
        outputs.insert("report".to_string(), summary);

        if let Some(path) = output_html {
            let body = build_branded_html_report(
                "Local Moran's I (LISA)",
                &[
                    "HH",
                    "LL",
                    "HL",
                    "LH",
                    "NS",
                    "N used",
                    "N dropped",
                    "N islands",
                    "alpha",
                    "multiple testing",
                ],
                &[
                    hh.to_string(),
                    ll.to_string(),
                    hl.to_string(),
                    lh.to_string(),
                    ns.to_string(),
                    n_features_used.to_string(),
                    weights.diagnostics.dropped_feature_count.to_string(),
                    island_count.to_string(),
                    format!("{alpha:.6}"),
                    match multiple_testing {
                        MultipleTestingMode::None => "none".to_string(),
                        MultipleTestingMode::FdrBh => "fdr_bh".to_string(),
                        MultipleTestingMode::Bonferroni => "bonferroni".to_string(),
                    },
                ],
            );
            write_text(&path, &body)?;
            outputs.insert("output_html".to_string(), json!(path.to_string_lossy().to_string()));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

#[derive(Clone, Copy)]
enum GiVariant {
    Gi,
    GiStar,
}

impl GiVariant {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError> {
        let text = args
            .get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("gi_star")
            .trim()
            .to_ascii_lowercase();
        match text.as_str() {
            "gi" => Ok(Self::Gi),
            "gi_star" => Ok(Self::GiStar),
            _ => Err(ToolError::Validation(
                "variant must be one of: gi, gi_star".to_string(),
            )),
        }
    }
}

impl Tool for GetisOrdGiStarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "getis_ord_gi_star",
            display_name: "Getis-Ord Gi / Gi*",
            summary: "Computes Getis-Ord Gi or Gi* z-scores and hotspot/coldspot classes.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field to analyze.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode.", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "variant", description: "Variant: gi or gi_star (default gi_star).", required: false },
                ToolParamSpec { name: "inference", description: "Inference mode: asymptotic (current) or permutation (future).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "alpha", description: "Significance threshold in [0, 1]; default 0.05.", required: false },
                ToolParamSpec { name: "multiple_testing", description: "Multiple-testing correction: none, fdr_bh, bonferroni.", required: false },
                ToolParamSpec { name: "output", description: "Output vector path with GI fields.", required: true },
                ToolParamSpec { name: "output_html", description: "Optional HTML report output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("weights_mode".to_string(), json!("k_nearest"));
        defaults.insert("k".to_string(), json!(8));
        defaults.insert("row_standardize".to_string(), json!(true));
        defaults.insert("variant".to_string(), json!("gi_star"));
        defaults.insert("inference".to_string(), json!("asymptotic"));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));
        defaults.insert("alpha".to_string(), json!(0.05));
        defaults.insert("multiple_testing".to_string(), json!("fdr_bh"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("gi_star_output.gpkg"));

        ToolManifest {
            id: "getis_ord_gi_star".to_string(),
            display_name: "Getis-Ord Gi / Gi*".to_string(),
            summary: "Computes Getis-Ord Gi or Gi* z-scores and hotspot/coldspot classes.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field to analyze.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode: queen, rook, k_nearest, distance_band.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest mode.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold for distance_band mode.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Apply row standardization to weights (default true).".to_string(), required: false },
                ToolParamDescriptor { name: "variant".to_string(), description: "Variant: gi or gi_star (default gi_star).".to_string(), required: false },
                ToolParamDescriptor { name: "inference".to_string(), description: "Inference mode: asymptotic (current) or permutation (future).".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "alpha".to_string(), description: "Significance threshold in [0, 1]; default 0.05.".to_string(), required: false },
                ToolParamDescriptor { name: "multiple_testing".to_string(), description: "Multiple-testing correction: none, fdr_bh, bonferroni.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path with GI fields.".to_string(), required: true },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "getis_ord_gi_star_basic".to_string(),
                description: "Computes Gi* and writes per-feature hotspot classes.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "spatial-statistics".to_string(),
                "hotspot".to_string(),
                "coldspot".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        if field.trim().is_empty() {
            return Err(ToolError::Validation("field must be non-empty".to_string()));
        }

        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        if matches!(mode, SpatialWeightsMode::KNearest) && k == 0 {
            return Err(ToolError::Validation("k must be > 0".to_string()));
        }
        if matches!(mode, SpatialWeightsMode::DistanceBand) {
            let d = parse_f64_arg(args, "distance")?;
            if !d.is_finite() || d <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }
        if let Some(distance) = parse_optional_f64_arg(args, "distance") {
            if !distance.is_finite() || distance <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }

        let inference = args
            .get("inference")
            .and_then(|v| v.as_str())
            .unwrap_or("asymptotic")
            .trim()
            .to_ascii_lowercase();
        if inference != "asymptotic" && inference != "permutation" {
            return Err(ToolError::Validation(
                "inference must be one of: asymptotic, permutation".to_string(),
            ));
        }

        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(ToolError::Validation("alpha must be in [0, 1]".to_string()));
        }

        let _ = IslandPolicy::parse(args)?;
        let _ = GiVariant::parse(args)?;
        let _ = MultipleTestingMode::parse(args)?;
        let _ = parse_vector_path_arg(args, "output")?;
        let _ = parse_optional_output_path(args, "output_html")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let variant = GiVariant::parse(args)?;
        let inference = args
            .get("inference")
            .and_then(|v| v.as_str())
            .unwrap_or("asymptotic")
            .trim()
            .to_ascii_lowercase();
        let island_policy = IslandPolicy::parse(args)?;
        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        let multiple_testing = MultipleTestingMode::parse(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let output_html = parse_optional_output_path(args, "output_html")?;

        if inference == "permutation" {
            return Err(ToolError::Validation(
                "permutation inference is not implemented yet for getis_ord_gi_star; use inference='asymptotic'"
                    .to_string(),
            ));
        }

        let (observations, dropped) = collect_spatial_observations(&input, &field)?;
        let values: Vec<f64> = observations.iter().map(|o| o.value).collect();

        ctx.progress.info("building spatial weights");
        let weights = build_spatial_weights(
            &observations,
            mode,
            row_standardize,
            island_policy,
            k,
            distance,
            dropped,
        )?;

        ctx.progress.info("computing Getis-Ord G*");
        let (gi_z, gi_p, cluster_type) = compute_getis_ord_gi_star(&values, &weights, island_policy, alpha)?;

        // Count islands for reporting
        let n_obs = observations.len();
        let mut island_count = 0usize;
        if matches!(island_policy, IslandPolicy::DropWithWarning) {
            for i in 0..n_obs {
                if weights.neighbors[i].is_empty() {
                    island_count += 1;
                }
            }
        }

        let gi_p_adj = adjust_p_values(&gi_p, multiple_testing);

        let mut output = input.clone();
        let mut schema = output.schema.clone();
        for field_name in ["GI_Z", "GI_P", "GI_P_ADJ", "GI_SIG", "GI_CLASS"] {
            if schema.field_index(field_name).is_some() {
                return Err(ToolError::Validation(format!(
                    "output schema already contains field '{}'", field_name
                )));
            }
        }
        schema.add_field(wbvector::FieldDef::new("GI_Z", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("GI_P", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("GI_P_ADJ", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("GI_SIG", wbvector::FieldType::Integer));
        schema.add_field(wbvector::FieldDef::new("GI_CLASS", wbvector::FieldType::Text));
        output.schema = schema;

        let mut obs_by_source = vec![None; input.features.len()];
        for (obs_idx, obs) in observations.iter().enumerate() {
            obs_by_source[obs.source_index] = Some(obs_idx);
        }

        let mut hot = 0usize;
        let mut cold = 0usize;
        let mut ns = 0usize;

        for feature_index in 0..output.features.len() {
            if let Some(obs_idx) = obs_by_source[feature_index] {
                let z_value = gi_z[obs_idx];
                let p_adj = gi_p_adj[obs_idx];
                let sig = p_adj.is_some_and(|p| p <= alpha);
                let class_str = &cluster_type[obs_idx];
                let class = if sig {
                    match class_str.as_str() {
                        "HotSpot" => "hot",
                        "ColdSpot" => "cold",
                        _ => "ns",
                    }
                } else {
                    "ns"
                };
                match class {
                    "hot" => hot += 1,
                    "cold" => cold += 1,
                    _ => ns += 1,
                }

                output.features[feature_index]
                    .attributes
                    .push(z_value.map_or(wbvector::FieldValue::Null, wbvector::FieldValue::Float));
                output.features[feature_index]
                    .attributes
                    .push(gi_p[obs_idx].map_or(wbvector::FieldValue::Null, wbvector::FieldValue::Float));
                output.features[feature_index]
                    .attributes
                    .push(p_adj.map_or(wbvector::FieldValue::Null, wbvector::FieldValue::Float));
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Integer(if sig { 1 } else { 0 }));
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Text(class.to_string()));
            } else {
                ns += 1;
                output.features[feature_index].attributes.push(wbvector::FieldValue::Null);
                output.features[feature_index].attributes.push(wbvector::FieldValue::Null);
                output.features[feature_index].attributes.push(wbvector::FieldValue::Null);
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Integer(0));
                output.features[feature_index]
                    .attributes
                    .push(wbvector::FieldValue::Text("ns".to_string()));
            }
        }

        let locator = write_vector_output(&output, output_path.trim())?;

        let n_features_used = n_obs - weights.diagnostics.dropped_feature_count - island_count;

        let summary = json!({
                "tool_id": "getis_ord_gi_star",
                "inference_method": "asymptotic",
                "variant": match variant {
                    GiVariant::Gi => "gi",
                    GiVariant::GiStar => "gi_star",
                },
                "statistic": serde_json::Value::Null,
                "p_value": serde_json::Value::Null,
                "alpha": alpha,
                "significance_class": serde_json::Value::Null,
                "multiple_testing": match multiple_testing {
                    MultipleTestingMode::None => "none",
                    MultipleTestingMode::FdrBh => "fdr_bh",
                    MultipleTestingMode::Bonferroni => "bonferroni",
                },
                "n_features_used": n_features_used,
                "n_features_dropped": weights.diagnostics.dropped_feature_count,
                "n_observations": n_features_used,
                "dropped_observations": weights.diagnostics.dropped_feature_count,
                "n_islands": island_count,
                "class_counts": {
                    "hot": hot,
                    "cold": cold,
                    "ns": ns,
                },
                "weights_diagnostics": {
                    "n_features": weights.diagnostics.n_features,
                    "n_islands": weights.diagnostics.n_islands,
                    "neighbor_count_min": weights.diagnostics.neighbor_count_min,
                    "neighbor_count_mean": weights.diagnostics.neighbor_count_mean,
                    "neighbor_count_max": weights.diagnostics.neighbor_count_max,
                    "connected_component_count": weights.diagnostics.connected_component_count,
                    "row_standardized": weights.diagnostics.row_standardized,
                },
                "warnings": weights.warnings,
                "assumption_flags": {
                    "permutation_supported": false,
                    "inference": "asymptotic",
                },
                "runtime_metadata": {
                    "seed": serde_json::Value::Null,
                    "permutations": serde_json::Value::Null,
                },
            });

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), json!(locator));
        outputs.insert("summary".to_string(), summary.clone());
        outputs.insert("report".to_string(), summary);

        if let Some(path) = output_html {
            let body = build_branded_html_report(
                "Getis-Ord Gi / Gi*",
                &[
                    "hot",
                    "cold",
                    "ns",
                    "N used",
                    "N dropped",
                    "N islands",
                    "alpha",
                    "variant",
                    "multiple testing",
                ],
                &[
                    hot.to_string(),
                    cold.to_string(),
                    ns.to_string(),
                    n_features_used.to_string(),
                    weights.diagnostics.dropped_feature_count.to_string(),
                    island_count.to_string(),
                    format!("{alpha:.6}"),
                    match variant {
                        GiVariant::Gi => "gi".to_string(),
                        GiVariant::GiStar => "gi_star".to_string(),
                    },
                    match multiple_testing {
                        MultipleTestingMode::None => "none".to_string(),
                        MultipleTestingMode::FdrBh => "fdr_bh".to_string(),
                        MultipleTestingMode::Bonferroni => "bonferroni".to_string(),
                    },
                ],
            );
            write_text(&path, &body)?;
            outputs.insert("output_html".to_string(), json!(path.to_string_lossy().to_string()));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

#[derive(Clone, Copy)]
enum StudyAreaMode {
    Hull,
    Envelope,
    PolygonLayer,
}

impl StudyAreaMode {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError> {
        let text = args
            .get("study_area_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("hull")
            .trim()
            .to_ascii_lowercase();
        match text.as_str() {
            "hull" => Ok(Self::Hull),
            "envelope" => Ok(Self::Envelope),
            "polygon_layer" => Ok(Self::PolygonLayer),
            _ => Err(ToolError::Validation(
                "study_area_mode must be one of: hull, envelope, polygon_layer".to_string(),
            )),
        }
    }
}

#[derive(Clone, Copy)]
enum QuadratGridMode {
    RowsCols,
    CellSize,
}

impl QuadratGridMode {
    fn parse(args: &ToolArgs) -> Result<Self, ToolError> {
        let text = args
            .get("grid_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("rows_cols")
            .trim()
            .to_ascii_lowercase();
        match text.as_str() {
            "rows_cols" => Ok(Self::RowsCols),
            "cell_size" => Ok(Self::CellSize),
            _ => Err(ToolError::Validation(
                "grid_mode must be one of: rows_cols, cell_size".to_string(),
            )),
        }
    }
}

fn collect_input_points(layer: &wbvector::Layer) -> Result<Vec<(f64, f64)>, ToolError> {
    let mut points = Vec::<(f64, f64)>::new();
    for feature in &layer.features {
        let Some(geometry) = &feature.geometry else {
            continue;
        };
        let mut coords = Vec::<&wbvector::Coord>::new();
        super::collect_geometry_coords(geometry, &mut coords);
        for c in coords {
            points.push((c.x, c.y));
        }
    }
    if points.len() < 2 {
        return Err(ToolError::Validation(
            "input must contain at least two point samples".to_string(),
        ));
    }
    Ok(points)
}

fn points_envelope(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in points {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }
    (min_x, min_y, max_x, max_y)
}

fn convex_hull_area(points: &[(f64, f64)]) -> f64 {
    let topo_points: Vec<TopoCoord> = points
        .iter()
        .map(|(x, y)| TopoCoord::xy(*x, *y))
        .collect();
    match convex_hull(&topo_points, 1.0e-12) {
        TopoGeometry::Polygon(poly) => geometry_area(&TopoGeometry::Polygon(poly)).abs(),
        _ => {
            let (min_x, min_y, max_x, max_y) = points_envelope(points);
            ((max_x - min_x).abs() * (max_y - min_y).abs()).max(1.0e-12)
        }
    }
}

fn polygon_area_and_membership(
    polygons_layer: &wbvector::Layer,
) -> Result<(f64, Vec<(wbvector::Ring, Vec<wbvector::Ring>)>), ToolError> {
    let polygons = super::collect_layer_polygons(polygons_layer)?;
    let mut area = 0.0f64;
    for (exterior, interiors) in &polygons {
        let poly = super::to_topo_polygon(exterior, interiors);
        area += geometry_area(&TopoGeometry::Polygon(poly)).abs();
    }
    if area <= 0.0 {
        return Err(ToolError::Validation(
            "study_area_polygon has non-positive area".to_string(),
        ));
    }
    Ok((area, polygons))
}


impl Tool for NearestNeighbourIndexTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "nearest_neighbour_index",
            display_name: "Nearest Neighbour Index",
            summary: "Computes the Clark-Evans nearest-neighbour index with asymptotic inference.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "study_area_mode", description: "Study area mode: hull, envelope, polygon_layer.", required: false },
                ToolParamSpec { name: "study_area_polygon", description: "Polygon layer used when study_area_mode=polygon_layer.", required: false },
                ToolParamSpec { name: "output_json", description: "Optional JSON report output path.", required: false },
                ToolParamSpec { name: "output_html", description: "Optional HTML report output path.", required: false },
                ToolParamSpec { name: "output_csv", description: "Optional CSV summary output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.gpkg"));
        defaults.insert("study_area_mode".to_string(), json!("hull"));

        let mut example_args = defaults.clone();
        example_args.insert("output_json".to_string(), json!("nni_report.json"));

        ToolManifest {
            id: "nearest_neighbour_index".to_string(),
            display_name: "Nearest Neighbour Index".to_string(),
            summary: "Computes the Clark-Evans nearest-neighbour index with asymptotic inference.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "study_area_mode".to_string(), description: "Study area mode: hull, envelope, polygon_layer.".to_string(), required: false },
                ToolParamDescriptor { name: "study_area_polygon".to_string(), description: "Polygon layer used when study_area_mode=polygon_layer.".to_string(), required: false },
                ToolParamDescriptor { name: "output_json".to_string(), description: "Optional JSON report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_csv".to_string(), description: "Optional CSV summary output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "nearest_neighbour_index_basic".to_string(),
                description: "Computes NNI and writes a JSON report.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "spatial-statistics".to_string(), "point-pattern".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let mode = StudyAreaMode::parse(args)?;
        if matches!(mode, StudyAreaMode::PolygonLayer) {
            let _ = parse_required_vector_path_arg(args, "study_area_polygon")?;
        }
        let _ = parse_optional_output_path(args, "output_json")?;
        let _ = parse_optional_output_path(args, "output_html")?;
        let _ = parse_optional_output_path(args, "output_csv")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let mode = StudyAreaMode::parse(args)?;
        let points_all = collect_input_points(&input)?;

        let (study_area, points): (f64, Vec<(f64, f64)>) = match mode {
            StudyAreaMode::Hull => (convex_hull_area(&points_all), points_all),
            StudyAreaMode::Envelope => {
                let (min_x, min_y, max_x, max_y) = points_envelope(&points_all);
                (((max_x - min_x).abs() * (max_y - min_y).abs()).max(1.0e-12), points_all)
            }
            StudyAreaMode::PolygonLayer => {
                let study_path = parse_required_vector_path_arg(args, "study_area_polygon")?;
                let polygons_layer = wbvector::read(&study_path)
                    .map_err(|e| ToolError::Execution(format!("failed reading study_area_polygon: {e}")))?;
                let (area, polygons) = polygon_area_and_membership(&polygons_layer)?;
                let filtered: Vec<(f64, f64)> = points_all
                    .into_iter()
                    .filter(|(x, y)| {
                        polygons.iter().any(|(exterior, interiors)| {
                            polygon_contains_xy(exterior, interiors, *x, *y)
                        })
                    })
                    .collect();
                (area, filtered)
            }
        };

        if points.len() < 2 {
            return Err(ToolError::Validation(
                "nearest_neighbour_index requires at least two points in the study area"
                    .to_string(),
            ));
        }

        ctx.progress.info("computing nearest-neighbour index");
        let result = autocorrelation::nearest_neighbor_index(&points)
            .map_err(|e| ToolError::Validation(format!("NNI computation failed: {}", e)))?;

        let observed_mean = result.observed_distance;
        let expected_mean = result.expected_distance;
        let nni_ratio = result.nni;
        let z_score = result.z_score;
        let p_value = result.p_value;

        let significance_class = if p_value <= 0.05 {
            if z_score > 0.0 { "clustered" } else { "dispersed" }
        } else {
            "ns"
        };

        let report = json!({
            "tool_id": "nearest_neighbour_index",
            "inference_method": "asymptotic",
            "statistic": nni_ratio,
            "p_value": p_value,
            "alpha": 0.05,
            "significance_class": significance_class,
            "observed_mean_distance": observed_mean,
            "expected_mean_distance_csr": expected_mean,
            "nni_ratio": nni_ratio,
            "z_score": z_score,
            "p_value_two_sided": p_value,
            "n_points": points.len(),
            "n_observations": points.len(),
            "dropped_observations": 0,
            "study_area": study_area,
            "study_area_mode": match mode {
                StudyAreaMode::Hull => "hull",
                StudyAreaMode::Envelope => "envelope",
                StudyAreaMode::PolygonLayer => "polygon_layer",
            },
            "weights_diagnostics": serde_json::Value::Null,
            "warnings": [],
            "assumption_flags": {
                "distance_metric": "euclidean",
                "inference": "asymptotic",
            },
            "runtime_metadata": {
                "seed": serde_json::Value::Null,
                "permutations": serde_json::Value::Null,
            },
        });

        let output_json = parse_optional_output_path(args, "output_json")?;
        let output_html = parse_optional_output_path(args, "output_html")?;
        let output_csv = parse_optional_output_path(args, "output_csv")?;

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), report.clone());
        outputs.insert("summary".to_string(), report.clone());

        if let Some(path) = output_json {
            let body = serde_json::to_string_pretty(&report)
                .map_err(|e| ToolError::Execution(format!("failed serializing JSON report: {e}")))?;
            write_text(&path, &body)?;
            outputs.insert("output_json".to_string(), json!(path.to_string_lossy().to_string()));
        }
        if let Some(path) = output_csv {
            let body = format!(
                "tool_id,observed_mean_distance,expected_mean_distance_csr,nni_ratio,z_score,p_value_two_sided,n_points,study_area\nnearest_neighbour_index,{},{},{},{},{},{},{}\n",
                observed_mean,
                expected_mean,
                nni_ratio,
                z_score,
                p_value,
                points.len(),
                study_area
            );
            write_text(&path, &body)?;
            outputs.insert("output_csv".to_string(), json!(path.to_string_lossy().to_string()));
        }
        if let Some(path) = output_html {
            let body = build_branded_html_report(
                "Nearest Neighbour Index",
                &[
                    "Observed Mean",
                    "Expected Mean (CSR)",
                    "NNI",
                    "Z",
                    "P",
                    "N points",
                    "Study area",
                ],
                &[
                    format!("{observed_mean:.6}"),
                    format!("{expected_mean:.6}"),
                    format!("{nni_ratio:.6}"),
                    format!("{z_score:.6}"),
                    format!("{p_value:.6}"),
                    points.len().to_string(),
                    format!("{study_area:.6}"),
                ],
            );
            write_text(&path, &body)?;
            outputs.insert("output_html".to_string(), json!(path.to_string_lossy().to_string()));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for QuadratCountTestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "quadrat_count_test",
            display_name: "Quadrat Count Test",
            summary: "Runs a quadrat count chi-square test for point-pattern randomness.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "grid_mode", description: "Grid mode: rows_cols or cell_size.", required: false },
                ToolParamSpec { name: "rows", description: "Rows when grid_mode=rows_cols.", required: false },
                ToolParamSpec { name: "cols", description: "Cols when grid_mode=rows_cols.", required: false },
                ToolParamSpec { name: "cell_size", description: "Cell size when grid_mode=cell_size.", required: false },
                ToolParamSpec { name: "study_area_mode", description: "Study area mode: hull, envelope, polygon_layer.", required: false },
                ToolParamSpec { name: "study_area_polygon", description: "Polygon layer used when study_area_mode=polygon_layer.", required: false },
                ToolParamSpec { name: "output_grid", description: "Optional quadrat polygon grid output path.", required: false },
                ToolParamSpec { name: "output_json", description: "Optional JSON report output path.", required: false },
                ToolParamSpec { name: "output_html", description: "Optional HTML report output path.", required: false },
                ToolParamSpec { name: "output_csv", description: "Optional CSV summary output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.gpkg"));
        defaults.insert("grid_mode".to_string(), json!("rows_cols"));
        defaults.insert("rows".to_string(), json!(10));
        defaults.insert("cols".to_string(), json!(10));
        defaults.insert("study_area_mode".to_string(), json!("hull"));

        let mut example_args = defaults.clone();
        example_args.insert("output_json".to_string(), json!("quadrat_report.json"));

        ToolManifest {
            id: "quadrat_count_test".to_string(),
            display_name: "Quadrat Count Test".to_string(),
            summary: "Runs a quadrat count chi-square test for point-pattern randomness.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "grid_mode".to_string(), description: "Grid mode: rows_cols or cell_size.".to_string(), required: false },
                ToolParamDescriptor { name: "rows".to_string(), description: "Rows when grid_mode=rows_cols.".to_string(), required: false },
                ToolParamDescriptor { name: "cols".to_string(), description: "Cols when grid_mode=rows_cols.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Cell size when grid_mode=cell_size.".to_string(), required: false },
                ToolParamDescriptor { name: "study_area_mode".to_string(), description: "Study area mode: hull, envelope, polygon_layer.".to_string(), required: false },
                ToolParamDescriptor { name: "study_area_polygon".to_string(), description: "Polygon layer used when study_area_mode=polygon_layer.".to_string(), required: false },
                ToolParamDescriptor { name: "output_grid".to_string(), description: "Optional quadrat polygon grid output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_json".to_string(), description: "Optional JSON report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_csv".to_string(), description: "Optional CSV summary output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "quadrat_count_test_basic".to_string(),
                description: "Runs quadrat count test and writes JSON summary.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "spatial-statistics".to_string(), "point-pattern".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let mode = QuadratGridMode::parse(args)?;
        match mode {
            QuadratGridMode::RowsCols => {
                let rows = parse_optional_usize_arg(args, "rows")?.unwrap_or(10);
                let cols = parse_optional_usize_arg(args, "cols")?.unwrap_or(10);
                if rows == 0 || cols == 0 {
                    return Err(ToolError::Validation("rows and cols must be > 0".to_string()));
                }
            }
            QuadratGridMode::CellSize => {
                let cell_size = parse_f64_arg(args, "cell_size")?;
                if !cell_size.is_finite() || cell_size <= 0.0 {
                    return Err(ToolError::Validation("cell_size must be finite and > 0".to_string()));
                }
            }
        }

        let study_mode = StudyAreaMode::parse(args)?;
        if matches!(study_mode, StudyAreaMode::PolygonLayer) {
            let _ = parse_required_vector_path_arg(args, "study_area_polygon")?;
        }
        let _ = parse_optional_output_path(args, "output_grid")?;
        let _ = parse_optional_output_path(args, "output_json")?;
        let _ = parse_optional_output_path(args, "output_html")?;
        let _ = parse_optional_output_path(args, "output_csv")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let study_mode = StudyAreaMode::parse(args)?;
        let grid_mode = QuadratGridMode::parse(args)?;
        let points_all = collect_input_points(&input)?;

        let mut points = points_all.clone();
        let study_area = match study_mode {
            StudyAreaMode::Hull => convex_hull_area(&points_all),
            StudyAreaMode::Envelope => {
                let (min_x, min_y, max_x, max_y) = points_envelope(&points_all);
                ((max_x - min_x).abs() * (max_y - min_y).abs()).max(1.0e-12)
            }
            StudyAreaMode::PolygonLayer => {
                let study_path = parse_required_vector_path_arg(args, "study_area_polygon")?;
                let polygons_layer = wbvector::read(&study_path)
                    .map_err(|e| ToolError::Execution(format!("failed reading study_area_polygon: {e}")))?;
                let (area, polygons) = polygon_area_and_membership(&polygons_layer)?;
                points = points_all
                    .into_iter()
                    .filter(|(x, y)| {
                        polygons.iter().any(|(exterior, interiors)| {
                            polygon_contains_xy(exterior, interiors, *x, *y)
                        })
                    })
                    .collect();
                area
            }
        };

        if points.is_empty() {
            return Err(ToolError::Validation(
                "no points remain in the selected study area".to_string(),
            ));
        }

        let (min_x, min_y, max_x, max_y) = points_envelope(&points);
        let width = (max_x - min_x).max(1.0e-12);
        let height = (max_y - min_y).max(1.0e-12);

        let (rows, cols) = match grid_mode {
            QuadratGridMode::RowsCols => (
                parse_optional_usize_arg(args, "rows")?.unwrap_or(10),
                parse_optional_usize_arg(args, "cols")?.unwrap_or(10),
            ),
            QuadratGridMode::CellSize => {
                let cell_size = parse_f64_arg(args, "cell_size")?;
                (
                    (height / cell_size).ceil().max(1.0) as usize,
                    (width / cell_size).ceil().max(1.0) as usize,
                )
            }
        };

        let n_quadrats = rows * cols;
        let dx = width / cols as f64;
        let dy = height / rows as f64;

        let mut counts = vec![0usize; n_quadrats];
        for (x, y) in &points {
            let mut c = ((*x - min_x) / dx).floor() as isize;
            let mut r = ((*y - min_y) / dy).floor() as isize;
            if c < 0 {
                c = 0;
            }
            if r < 0 {
                r = 0;
            }
            if c >= cols as isize {
                c = cols as isize - 1;
            }
            if r >= rows as isize {
                r = rows as isize - 1;
            }
            let idx = r as usize * cols + c as usize;
            counts[idx] += 1;
        }

        let n_points = points.len() as f64;
        let expected = n_points / n_quadrats as f64;

        ctx.progress.info("computing quadrat analysis statistics");
        let result = autocorrelation::quadrat_analysis(&points, rows, cols)
            .map_err(|e| ToolError::Validation(format!("Quadrat analysis failed: {}", e)))?;

        let chi_square = result.chi_square;
        let df = result.degrees_of_freedom as f64;
        let p_value = result.p_value;
        let vmr = result.variance_mean_ratio;

        let significance_class = if p_value <= 0.05 { "non_random" } else { "ns" };

        let report = json!({
            "tool_id": "quadrat_count_test",
            "inference_method": "asymptotic",
            "statistic": chi_square,
            "p_value": p_value,
            "alpha": 0.05,
            "significance_class": significance_class,
            "chi_square": chi_square,
            "df": df as usize,
            "p_value": p_value,
            "variance_to_mean_ratio": vmr,
            "n_quadrats": n_quadrats,
            "n_points": points.len(),
            "n_observations": points.len(),
            "dropped_observations": 0,
            "study_area": study_area,
            "weights_diagnostics": serde_json::Value::Null,
            "warnings": [],
            "assumption_flags": {
                "grid_mode": args
                    .get("grid_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("rows_cols"),
                "inference": "asymptotic",
            },
            "runtime_metadata": {
                "seed": serde_json::Value::Null,
                "permutations": serde_json::Value::Null,
            },
        });

        let output_grid = parse_optional_output_path(args, "output_grid")?;
        let output_json = parse_optional_output_path(args, "output_json")?;
        let output_html = parse_optional_output_path(args, "output_html")?;
        let output_csv = parse_optional_output_path(args, "output_csv")?;

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), report.clone());
        outputs.insert("summary".to_string(), report.clone());

        if let Some(path) = output_json {
            let body = serde_json::to_string_pretty(&report)
                .map_err(|e| ToolError::Execution(format!("failed serializing JSON report: {e}")))?;
            write_text(&path, &body)?;
            outputs.insert("output_json".to_string(), json!(path.to_string_lossy().to_string()));
        }
        if let Some(path) = output_csv {
            let body = format!(
                "tool_id,chi_square,df,p_value,variance_to_mean_ratio,n_quadrats,n_points,study_area\nquadrat_count_test,{},{},{},{},{},{},{}\n",
                chi_square,
                df as usize,
                p_value,
                vmr,
                n_quadrats,
                points.len(),
                study_area,
            );
            write_text(&path, &body)?;
            outputs.insert("output_csv".to_string(), json!(path.to_string_lossy().to_string()));
        }
        if let Some(path) = output_html {
            let body = build_branded_html_report(
                "Quadrat Count Test",
                &[
                    "Chi-square",
                    "df",
                    "P",
                    "VMR",
                    "N quadrats",
                    "N points",
                    "Study area",
                ],
                &[
                    format!("{chi_square:.6}"),
                    (df as usize).to_string(),
                    format!("{p_value:.6}"),
                    format!("{vmr:.6}"),
                    n_quadrats.to_string(),
                    points.len().to_string(),
                    format!("{study_area:.6}"),
                ],
            );
            write_text(&path, &body)?;
            outputs.insert("output_html".to_string(), json!(path.to_string_lossy().to_string()));
        }

        if let Some(path) = output_grid {
            let mut grid = wbvector::Layer::new("quadrat_grid")
                .with_geom_type(wbvector::GeometryType::Polygon);
            grid.crs = input.crs.clone();
            grid.schema.add_field(wbvector::FieldDef::new("ROW", wbvector::FieldType::Integer));
            grid.schema.add_field(wbvector::FieldDef::new("COL", wbvector::FieldType::Integer));
            grid.schema.add_field(wbvector::FieldDef::new("COUNT", wbvector::FieldType::Integer));
            grid.schema.add_field(wbvector::FieldDef::new("EXPECTED", wbvector::FieldType::Float));

            for r in 0..rows {
                for c in 0..cols {
                    let x0 = min_x + c as f64 * dx;
                    let x1 = x0 + dx;
                    let y0 = min_y + r as f64 * dy;
                    let y1 = y0 + dy;
                    let ring = wbvector::Ring::new(vec![
                        wbvector::Coord::xy(x0, y0),
                        wbvector::Coord::xy(x1, y0),
                        wbvector::Coord::xy(x1, y1),
                        wbvector::Coord::xy(x0, y1),
                        wbvector::Coord::xy(x0, y0),
                    ]);
                    let idx = r * cols + c;
                    grid
                        .add_feature(
                            Some(wbvector::Geometry::Polygon {
                                exterior: ring,
                                interiors: Vec::new(),
                            }),
                            &[
                                ("ROW", wbvector::FieldValue::Integer(r as i64)),
                                ("COL", wbvector::FieldValue::Integer(c as i64)),
                                ("COUNT", wbvector::FieldValue::Integer(counts[idx] as i64)),
                                ("EXPECTED", wbvector::FieldValue::Float(expected)),
                            ],
                        )
                        .map_err(|e| ToolError::Execution(format!("failed creating quadrat grid feature: {e}")))?;
                }
            }

            let locator = write_vector_output(&grid, path.to_string_lossy().as_ref())?;
            outputs.insert("output_grid".to_string(), json!(locator));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// SPATIAL REGRESSION TOOLS (SAR, SEM, GWR) - Production integration pending
// ============================================================================

pub struct SpatialLagRegressionTool;
pub struct SpatialErrorRegressionTool;
pub struct GeographicallyWeightedRegressionTool;

impl Tool for SpatialLagRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spatial_lag_regression",
            display_name: "Spatial Lag Regression (SAR)",
            summary: "Estimates spatial lag regression model with GMM/IV+FGLS. Adds global coefficients and diagnostics.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable (dependent variable).", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor field names.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode.", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default: true).", required: false },
                ToolParamSpec { name: "output", description: "Output vector layer with regression results.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1,predictor2"));
        defaults.insert("weights_mode".to_string(), json!("queen"));
        defaults.insert("row_standardize".to_string(), json!(true));
        defaults.insert("output".to_string(), json!("output.gpkg"));

        ToolManifest {
            id: "spatial_lag_regression".to_string(),
            display_name: "Spatial Lag Regression (SAR)".to_string(),
            summary: "Estimates spatial lag regression with GMM/IV+FGLS framework.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response (dependent) variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Comma-separated predictor field names.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Spatial neighborhood mode.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Row standardize weights.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector layer.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "sar_basic".to_string(),
                description: "Estimate spatial lag regression with queen neighborhood.".to_string(),
                args: defaults,
            }],
            tags: vec![
                "vector".to_string(),
                "spatial-regression".to_string(),
                "sar".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "response_field")?;
        let _ = parse_string_arg(args, "predictor_fields")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        use wbspatialstats::regression::SpatialLagRegression;
        use nalgebra::DMatrix;

        let input = load_vector_arg(args, "input")?;
        let response_field = parse_string_arg(args, "response_field")?;
        let predictor_str = parse_string_arg(args, "predictor_fields")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let island_policy = IslandPolicy::parse(args)?;

        let predictor_fields: Vec<&str> = predictor_str.split(',').map(|s| s.trim()).collect();

        ctx.progress.info("Extracting response and predictor variables");
        let (observations, dropped) = collect_spatial_observations(&input, &response_field)?;
        if observations.is_empty() {
            return Err(ToolError::Execution(format!(
                "No valid observations after dropping {} features",
                dropped
            )));
        }

        let n = observations.len();
        let y: Vec<f64> = observations.iter().map(|o| o.value).collect();

        // Extract predictor fields - build design matrix column by column
        let mut x_data: Vec<f64> = Vec::with_capacity(n * (1 + predictor_fields.len()));
        
        // Intercept column
        for _ in 0..n {
            x_data.push(1.0);
        }

        for &pred_field in &predictor_fields {
            let (pred_obs, _) = collect_spatial_observations(&input, pred_field)?;
            if pred_obs.len() != n {
                return Err(ToolError::Execution(format!(
                    "Predictor '{}' has {} observations vs {} for response",
                    pred_field,
                    pred_obs.len(),
                    n
                )));
            }
            for obs in pred_obs {
                x_data.push(obs.value);
            }
        }

        let x = DMatrix::from_column_slice(n, 1 + predictor_fields.len(), &x_data);

        ctx.progress.info("Building spatial weights");
        let weights = build_spatial_weights(
            &observations,
            mode,
            row_standardize,
            island_policy,
            k,
            distance,
            dropped,
        )?;

        ctx.progress.info("Estimating spatial lag model (SAR)");
        let result = SpatialLagRegression::estimate(&y, &x, &weights, 100, 1e-6)
            .map_err(|e| ToolError::Execution(format!("SAR estimation failed: {}", e)))?;

        ctx.progress.info("Building output layer");
        let mut output_layer = input.clone();
        let mut schema = output_layer.schema.clone();

        // Add coefficient, SE, t-stat, p-value columns
        schema.add_field(wbvector::FieldDef::new("coef_intercept", wbvector::FieldType::Float));
        for pred_field in &predictor_fields {
            schema.add_field(wbvector::FieldDef::new(
                &format!("{}_coef", pred_field),
                wbvector::FieldType::Float,
            ));
            schema.add_field(wbvector::FieldDef::new(
                &format!("{}_se", pred_field),
                wbvector::FieldType::Float,
            ));
        }

        schema.add_field(wbvector::FieldDef::new("rho", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("rho_pvalue", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("r_squared", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("aic", wbvector::FieldType::Float));

        output_layer.schema = schema;

        // Add output features with results
        for (idx, feature) in input.features.iter().enumerate() {
            if idx >= n { break; }
            
            let mut new_feature = feature.clone();
            new_feature.attributes.insert(
                output_layer.schema.field_index("coef_intercept").unwrap(),
                wbvector::FieldValue::Float(result.base.coefficients[0]),
            );

            for (i, &pred_field) in predictor_fields.iter().enumerate() {
                let coef_idx = output_layer.schema.field_index(&format!("{}_coef", pred_field)).unwrap();
                let se_idx = output_layer.schema.field_index(&format!("{}_se", pred_field)).unwrap();

                new_feature.attributes.insert(
                    coef_idx,
                    wbvector::FieldValue::Float(result.base.coefficients[i + 1]),
                );
                new_feature.attributes.insert(
                    se_idx,
                    wbvector::FieldValue::Float(result.base.standard_errors[i + 1]),
                );
            }

            new_feature.attributes.insert(
                output_layer.schema.field_index("rho").unwrap(),
                wbvector::FieldValue::Float(result.rho),
            );
            new_feature.attributes.insert(
                output_layer.schema.field_index("rho_pvalue").unwrap(),
                wbvector::FieldValue::Float(result.rho_pvalue),
            );
            new_feature.attributes.insert(
                output_layer.schema.field_index("r_squared").unwrap(),
                wbvector::FieldValue::Float(result.base.r_squared),
            );
            new_feature.attributes.insert(
                output_layer.schema.field_index("aic").unwrap(),
                wbvector::FieldValue::Float(result.base.aic),
            );

            output_layer.features.push(new_feature);
        }

        let locator = write_vector_output(&output_layer, output_path.trim())?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for SpatialErrorRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spatial_error_regression",
            display_name: "Spatial Error Regression (SEM)",
            summary: "Estimates spatial error regression model with FGLS. Adds coefficients and diagnostics.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor field names.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest.", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Row standardize weights (default: true).", required: false },
                ToolParamSpec { name: "output", description: "Output vector layer with results.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1,predictor2"));
        defaults.insert("weights_mode".to_string(), json!("queen"));
        defaults.insert("row_standardize".to_string(), json!(true));
        defaults.insert("output".to_string(), json!("output.gpkg"));

        ToolManifest {
            id: "spatial_error_regression".to_string(),
            display_name: "Spatial Error Regression (SEM)".to_string(),
            summary: "Estimates spatial error regression with FGLS.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Comma-separated predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Spatial neighborhood mode.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k for k_nearest.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Row standardize weights.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector layer.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "sem_basic".to_string(),
                description: "Estimate spatial error regression with queen neighborhood.".to_string(),
                args: defaults,
            }],
            tags: vec![
                "vector".to_string(),
                "spatial-regression".to_string(),
                "sem".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "response_field")?;
        let _ = parse_string_arg(args, "predictor_fields")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        use wbspatialstats::regression::SpatialErrorRegression;
        use nalgebra::DMatrix;

        let input = load_vector_arg(args, "input")?;
        let response_field = parse_string_arg(args, "response_field")?;
        let predictor_str = parse_string_arg(args, "predictor_fields")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let island_policy = IslandPolicy::parse(args)?;

        let predictor_fields: Vec<&str> = predictor_str.split(',').map(|s| s.trim()).collect();

        ctx.progress.info("Extracting response and predictor variables");
        let (observations, dropped) = collect_spatial_observations(&input, &response_field)?;
        if observations.is_empty() {
            return Err(ToolError::Execution(format!(
                "No valid observations after dropping {} features",
                dropped
            )));
        }

        let n = observations.len();
        let y: Vec<f64> = observations.iter().map(|o| o.value).collect();

        // Extract predictor fields - build design matrix column by column
        let mut x_data: Vec<f64> = Vec::with_capacity(n * (1 + predictor_fields.len()));
        
        // Intercept column
        for _ in 0..n {
            x_data.push(1.0);
        }

        for &pred_field in &predictor_fields {
            let (pred_obs, _) = collect_spatial_observations(&input, pred_field)?;
            if pred_obs.len() != n {
                return Err(ToolError::Execution(format!(
                    "Predictor '{}' has {} observations vs {} for response",
                    pred_field,
                    pred_obs.len(),
                    n
                )));
            }
            for obs in pred_obs {
                x_data.push(obs.value);
            }
        }

        let x = DMatrix::from_column_slice(n, 1 + predictor_fields.len(), &x_data);

        ctx.progress.info("Building spatial weights");
        let weights = build_spatial_weights(
            &observations,
            mode,
            row_standardize,
            island_policy,
            k,
            distance,
            dropped,
        )?;

        ctx.progress.info("Estimating spatial error model (SEM)");
        let result = SpatialErrorRegression::estimate_fgls(&y, &x, &weights, 100, 1e-6)
            .map_err(|e| ToolError::Execution(format!("SEM estimation failed: {}", e)))?;

        ctx.progress.info("Building output layer");
        let mut output_layer = input.clone();
        let mut schema = output_layer.schema.clone();

        // Add coefficient, SE columns
        schema.add_field(wbvector::FieldDef::new("coef_intercept", wbvector::FieldType::Float));
        for pred_field in &predictor_fields {
            schema.add_field(wbvector::FieldDef::new(
                &format!("{}_coef", pred_field),
                wbvector::FieldType::Float,
            ));
            schema.add_field(wbvector::FieldDef::new(
                &format!("{}_se", pred_field),
                wbvector::FieldType::Float,
            ));
        }

        schema.add_field(wbvector::FieldDef::new("lambda", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("lambda_pvalue", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("r_squared", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("aic", wbvector::FieldType::Float));

        output_layer.schema = schema;

        // Add output features with results
        for (idx, feature) in input.features.iter().enumerate() {
            if idx >= n { break; }
            
            let mut new_feature = feature.clone();
            new_feature.attributes.insert(
                output_layer.schema.field_index("coef_intercept").unwrap(),
                wbvector::FieldValue::Float(result.base.coefficients[0]),
            );

            for (i, &pred_field) in predictor_fields.iter().enumerate() {
                let coef_idx = output_layer.schema.field_index(&format!("{}_coef", pred_field)).unwrap();
                let se_idx = output_layer.schema.field_index(&format!("{}_se", pred_field)).unwrap();

                new_feature.attributes.insert(
                    coef_idx,
                    wbvector::FieldValue::Float(result.base.coefficients[i + 1]),
                );
                new_feature.attributes.insert(
                    se_idx,
                    wbvector::FieldValue::Float(result.base.standard_errors[i + 1]),
                );
            }

            new_feature.attributes.insert(
                output_layer.schema.field_index("lambda").unwrap(),
                wbvector::FieldValue::Float(result.lambda),
            );
            new_feature.attributes.insert(
                output_layer.schema.field_index("lambda_pvalue").unwrap(),
                wbvector::FieldValue::Float(result.lambda_pvalue),
            );
            new_feature.attributes.insert(
                output_layer.schema.field_index("r_squared").unwrap(),
                wbvector::FieldValue::Float(result.base.r_squared),
            );
            new_feature.attributes.insert(
                output_layer.schema.field_index("aic").unwrap(),
                wbvector::FieldValue::Float(result.base.aic),
            );

            output_layer.features.push(new_feature);
        }

        let locator = write_vector_output(&output_layer, output_path.trim())?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for GeographicallyWeightedRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "geographically_weighted_regression",
            display_name: "Geographically Weighted Regression (GWR)",
            summary: "Estimates GWR with AICc-based bandwidth selection. Adds local coefficients per location.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor field names.", required: true },
                ToolParamSpec { name: "bandwidth_hint", description: "Optional bandwidth hint (auto-optimizes if omitted).", required: false },
                ToolParamSpec { name: "output", description: "Output vector with local coefficients.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1,predictor2"));
        defaults.insert("output".to_string(), json!("output.gpkg"));

        ToolManifest {
            id: "geographically_weighted_regression".to_string(),
            display_name: "Geographically Weighted Regression (GWR)".to_string(),
            summary: "Estimates GWR with AICc-optimized bandwidth selection.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Comma-separated predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "bandwidth_hint".to_string(), description: "Optional bandwidth hint.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector layer.".to_string(), required: true },
            ],
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "gwr_basic".to_string(),
                description: "Estimate GWR with automatic AICc bandwidth selection.".to_string(),
                args: defaults,
            }],
            tags: vec![
                "vector".to_string(),
                "spatial-regression".to_string(),
                "gwr".to_string(),
                "local-regression".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "response_field")?;
        let _ = parse_string_arg(args, "predictor_fields")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        use wbspatialstats::regression::GeographicallyWeightedRegression;
        use nalgebra::DMatrix;

        let input = load_vector_arg(args, "input")?;
        let response_field = parse_string_arg(args, "response_field")?;
        let predictor_str = parse_string_arg(args, "predictor_fields")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let bandwidth_hint = parse_optional_f64_arg(args, "bandwidth_hint");

        let predictor_fields: Vec<&str> = predictor_str.split(',').map(|s| s.trim()).collect();

        ctx.progress.info("Extracting response and predictor variables");
        let (observations, dropped) = collect_spatial_observations(&input, &response_field)?;
        if observations.is_empty() {
            return Err(ToolError::Execution(format!(
                "No valid observations after dropping {} features",
                dropped
            )));
        }

        let n = observations.len();
        let y: Vec<f64> = observations.iter().map(|o| o.value).collect();
        let coords: Vec<(f64, f64)> = observations.iter().map(|o| (o.x, o.y)).collect();

        // Extract predictor fields - build design matrix column by column
        let mut x_data: Vec<f64> = Vec::with_capacity(n * (1 + predictor_fields.len()));
        
        // Intercept column
        for _ in 0..n {
            x_data.push(1.0);
        }

        for &pred_field in &predictor_fields {
            let (pred_obs, _) = collect_spatial_observations(&input, pred_field)?;
            if pred_obs.len() != n {
                return Err(ToolError::Execution(format!(
                    "Predictor '{}' has {} observations vs {} for response",
                    pred_field,
                    pred_obs.len(),
                    n
                )));
            }
            for obs in pred_obs {
                x_data.push(obs.value);
            }
        }

        let x = DMatrix::from_column_slice(n, 1 + predictor_fields.len(), &x_data);

        ctx.progress.info("Estimating geographically weighted regression (GWR)");
        let result = GeographicallyWeightedRegression::estimate(&y, &x, &coords, bandwidth_hint)
            .map_err(|e| ToolError::Execution(format!("GWR estimation failed: {}", e)))?;

        ctx.progress.info("Building output layer");
        let mut output_layer = input.clone();
        let mut schema = output_layer.schema.clone();

        // Add local coefficient columns for each location and predictor
        schema.add_field(wbvector::FieldDef::new("coef_intercept_local", wbvector::FieldType::Float));
        for pred_field in &predictor_fields {
            schema.add_field(wbvector::FieldDef::new(
                &format!("{}_coef_local", pred_field),
                wbvector::FieldType::Float,
            ));
            schema.add_field(wbvector::FieldDef::new(
                &format!("{}_se_local", pred_field),
                wbvector::FieldType::Float,
            ));
        }

        schema.add_field(wbvector::FieldDef::new("gwr_bandwidth", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("gwr_r_squared", wbvector::FieldType::Float));

        output_layer.schema = schema;

        // Add output features with local coefficients
        for (idx, feature) in input.features.iter().enumerate() {
            if idx >= n { break; }
            
            let mut new_feature = feature.clone();
            
            new_feature.attributes.insert(
                output_layer.schema.field_index("coef_intercept_local").unwrap(),
                wbvector::FieldValue::Float(result.local_coefficients[(idx, 0)]),
            );

            for (i, &pred_field) in predictor_fields.iter().enumerate() {
                let coef_idx = output_layer.schema.field_index(&format!("{}_coef_local", pred_field)).unwrap();
                let se_idx = output_layer.schema.field_index(&format!("{}_se_local", pred_field)).unwrap();

                new_feature.attributes.insert(
                    coef_idx,
                    wbvector::FieldValue::Float(result.local_coefficients[(idx, i + 1)]),
                );
                new_feature.attributes.insert(
                    se_idx,
                    wbvector::FieldValue::Float(result.local_standard_errors[(idx, i + 1)]),
                );
            }

            new_feature.attributes.insert(
                output_layer.schema.field_index("gwr_bandwidth").unwrap(),
                wbvector::FieldValue::Float(result.bandwidth),
            );
            new_feature.attributes.insert(
                output_layer.schema.field_index("gwr_r_squared").unwrap(),
                wbvector::FieldValue::Float(result.r_squared),
            );

            output_layer.features.push(new_feature);
        }

        let locator = write_vector_output(&output_layer, output_path.trim())?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));
        
        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// PHASE A RASTER TOOLS
// ============================================================================

impl Tool for LocalMoransILisaRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "local_morans_i_lisa_raster",
            display_name: "Local Moran's I (LISA) - Raster Output",
            summary: "Computes Local Moran's I (LISA) and outputs a classification raster surface.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer with observation points.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field to analyze.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode (default 8).", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "alpha", description: "Significance threshold in [0, 1]; default 0.05.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output raster cell size (optional; uses input extent).", required: false },
                ToolParamSpec { name: "output", description: "Output raster path (classification: 0=NS, 1=HH, 2=LL, 3=HL, 4=LH).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("weights_mode".to_string(), json!("k_nearest"));
        defaults.insert("k".to_string(), json!(8));
        defaults.insert("row_standardize".to_string(), json!(true));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));
        defaults.insert("alpha".to_string(), json!(0.05));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("lisa_surface.tif"));

        ToolManifest {
            id: "local_morans_i_lisa_raster".to_string(),
            display_name: "Local Moran's I (LISA) - Raster Output".to_string(),
            summary: "Computes Local Moran's I (LISA) and outputs a classification raster surface.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer with observation points.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field to analyze.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode: queen, rook, k_nearest, distance_band.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest mode (default 8).".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold for distance_band mode.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Apply row standardization to weights (default true).".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "alpha".to_string(), description: "Significance threshold in [0, 1]; default 0.05.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output raster cell size (optional; auto-computed from extent).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output raster path (classification: 0=NS, 1=HH, 2=LL, 3=HL, 4=LH).".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "local_morans_i_lisa_raster_basic".to_string(),
                description: "Computes LISA and interpolates to a raster surface.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "spatial-statistics".to_string(),
                "autocorrelation".to_string(),
                "lisa".to_string(),
                "surface".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        if field.trim().is_empty() {
            return Err(ToolError::Validation("field must be non-empty".to_string()));
        }

        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        if matches!(mode, SpatialWeightsMode::KNearest) && k == 0 {
            return Err(ToolError::Validation("k must be > 0".to_string()));
        }
        if matches!(mode, SpatialWeightsMode::DistanceBand) {
            let d = parse_f64_arg(args, "distance")?;
            if !d.is_finite() || d <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }
        if let Some(distance) = parse_optional_f64_arg(args, "distance") {
            if !distance.is_finite() || distance <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }

        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(ToolError::Validation("alpha must be in [0, 1]".to_string()));
        }

        if let Some(cell_size) = parse_optional_f64_arg(args, "cell_size") {
            if !cell_size.is_finite() || cell_size <= 0.0 {
                return Err(ToolError::Validation("cell_size must be positive and finite".to_string()));
            }
        }

        let _ = IslandPolicy::parse(args)?;
        let _ = parse_raster_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let island_policy = IslandPolicy::parse(args)?;
        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        let cell_size = parse_optional_f64_arg(args, "cell_size");
        let output_path = parse_raster_path_arg(args, "output")?;

        ctx.progress.info("Extracting spatial observations");
        let (observations, dropped) = collect_spatial_observations(&input, &field)?;
        let values: Vec<f64> = observations.iter().map(|o| o.value).collect();

        ctx.progress.info("Building spatial weights");
        let weights = build_spatial_weights(
            &observations,
            mode,
            row_standardize,
            island_policy,
            k,
            distance,
            dropped,
        )?;

        ctx.progress.info("Computing LISA");
        let (_, _, _, quadrant) = compute_local_morans_i_lisa(&values, &weights, island_policy, alpha)?;

        ctx.progress.info("Building output raster");
        let samples: Vec<(f64, f64, f64)> = observations.iter().map(|o| (o.x, o.y, o.value)).collect();
        let mut output = super::build_point_interpolation_output(&input, &samples, cell_size, None, DataType::F64)?;

        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;

        ctx.progress.info("Interpolating LISA classes to raster grid");
        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                // Find nearest observation
                let mut nearest_idx = 0;
                let mut nearest_dist_sq = f64::INFINITY;
                for (idx, obs) in observations.iter().enumerate() {
                    let dx = obs.x - x;
                    let dy = obs.y - y;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < nearest_dist_sq {
                        nearest_dist_sq = dist_sq;
                        nearest_idx = idx;
                    }
                }

                // Map quadrant to classification value
                let class_value = match quadrant[nearest_idx].as_str() {
                    "HH" => 1.0,
                    "LL" => 2.0,
                    "HL" => 3.0,
                    "LH" => 4.0,
                    _ => 0.0, // "NS" and any other value maps to 0
                };

                let idx = row * cols + col;
                output.data.set_f64(idx, class_value);
            }

            let progress = (row as f64 + 1.0) / rows as f64;
            ctx.progress.progress(progress);
        }

        ctx.progress.info("Writing raster output");
        let locator = GisOverlayCore::store_or_write_output(output, output_path.trim(), ctx)?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for GetisOrdGiStarRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "getis_ord_gi_star_raster",
            display_name: "Getis-Ord Gi* - Raster Output",
            summary: "Computes Getis-Ord Gi* and outputs a hotspot/coldspot classification raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer with observation points.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field to analyze.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode (default 8).", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "alpha", description: "Significance threshold in [0, 1]; default 0.05.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output raster cell size (optional; uses input extent).", required: false },
                ToolParamSpec { name: "output", description: "Output raster path (classification: -1=Cold, 0=NS, 1=Hot).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("weights_mode".to_string(), json!("k_nearest"));
        defaults.insert("k".to_string(), json!(8));
        defaults.insert("row_standardize".to_string(), json!(true));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));
        defaults.insert("alpha".to_string(), json!(0.05));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("hotspots_surface.tif"));

        ToolManifest {
            id: "getis_ord_gi_star_raster".to_string(),
            display_name: "Getis-Ord Gi* - Raster Output".to_string(),
            summary: "Computes Getis-Ord Gi* and outputs a hotspot/coldspot classification raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer with observation points.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field to analyze.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode: queen, rook, k_nearest, distance_band.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest mode (default 8).".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold for distance_band mode.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Apply row standardization to weights (default true).".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "alpha".to_string(), description: "Significance threshold in [0, 1]; default 0.05.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output raster cell size (optional; auto-computed from extent).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output raster path (classification: -1=Cold, 0=NS, 1=Hot).".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "getis_ord_gi_star_raster_basic".to_string(),
                description: "Computes Gi* and interpolates hotspots/coldspots to a raster surface.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "spatial-statistics".to_string(),
                "hotspot".to_string(),
                "coldspot".to_string(),
                "surface".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        if field.trim().is_empty() {
            return Err(ToolError::Validation("field must be non-empty".to_string()));
        }

        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        if matches!(mode, SpatialWeightsMode::KNearest) && k == 0 {
            return Err(ToolError::Validation("k must be > 0".to_string()));
        }
        if matches!(mode, SpatialWeightsMode::DistanceBand) {
            let d = parse_f64_arg(args, "distance")?;
            if !d.is_finite() || d <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }
        if let Some(distance) = parse_optional_f64_arg(args, "distance") {
            if !distance.is_finite() || distance <= 0.0 {
                return Err(ToolError::Validation("distance must be finite and > 0".to_string()));
            }
        }

        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(ToolError::Validation("alpha must be in [0, 1]".to_string()));
        }

        if let Some(cell_size) = parse_optional_f64_arg(args, "cell_size") {
            if !cell_size.is_finite() || cell_size <= 0.0 {
                return Err(ToolError::Validation("cell_size must be positive and finite".to_string()));
            }
        }

        let _ = IslandPolicy::parse(args)?;
        let _ = parse_raster_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let field = parse_string_arg(args, "field")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let island_policy = IslandPolicy::parse(args)?;
        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        let cell_size = parse_optional_f64_arg(args, "cell_size");
        let output_path = parse_raster_path_arg(args, "output")?;

        ctx.progress.info("Extracting spatial observations");
        let (observations, dropped) = collect_spatial_observations(&input, &field)?;
        let values: Vec<f64> = observations.iter().map(|o| o.value).collect();

        ctx.progress.info("Building spatial weights");
        let weights = build_spatial_weights(
            &observations,
            mode,
            row_standardize,
            island_policy,
            k,
            distance,
            dropped,
        )?;

        ctx.progress.info("Computing Getis-Ord Gi*");
        let (_, _, cluster_type) = compute_getis_ord_gi_star(&values, &weights, island_policy, alpha)?;

        ctx.progress.info("Building output raster");
        let samples: Vec<(f64, f64, f64)> = observations.iter().map(|o| (o.x, o.y, o.value)).collect();
        let mut output = super::build_point_interpolation_output(&input, &samples, cell_size, None, DataType::F64)?;

        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;

        ctx.progress.info("Interpolating hotspot classes to raster grid");
        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                // Find nearest observation
                let mut nearest_idx = 0;
                let mut nearest_dist_sq = f64::INFINITY;
                for (idx, obs) in observations.iter().enumerate() {
                    let dx = obs.x - x;
                    let dy = obs.y - y;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < nearest_dist_sq {
                        nearest_dist_sq = dist_sq;
                        nearest_idx = idx;
                    }
                }

                // Map cluster type to classification value
                let class_value = match cluster_type[nearest_idx].as_str() {
                    "HotSpot" => 1.0,
                    "ColdSpot" => -1.0,
                    _ => 0.0, // "insignificant" and any other value maps to 0
                };

                let idx = row * cols + col;
                output.data.set_f64(idx, class_value);
            }

            let progress = (row as f64 + 1.0) / rows as f64;
            ctx.progress.progress(progress);
        }

        ctx.progress.info("Writing raster output");
        let locator = GisOverlayCore::store_or_write_output(output, output_path.trim(), ctx)?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// PHASE C RASTER TOOLS (Spatial Regression - Fitted Value Surfaces)
// ============================================================================
// TODO: Implement raster output versions of spatial regression tools
// These should estimate regression models and output fitted value surfaces

impl Tool for SpatialLagRegressionRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spatial_lag_regression_raster",
            display_name: "Spatial Lag Regression (SAR) - Raster Output",
            summary: "[NOT YET IMPLEMENTED] Outputs fitted value surface from spatial lag regression.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor fields.", required: true },
                ToolParamSpec { name: "output", description: "Output raster (fitted values).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1"));
        defaults.insert("output".to_string(), json!("fitted.tif"));

        ToolManifest {
            id: "spatial_lag_regression_raster".to_string(),
            display_name: "Spatial Lag Regression (SAR) - Raster Output".to_string(),
            summary: "[NOT YET IMPLEMENTED] Outputs fitted value surface from SAR model.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output raster.".to_string(), required: true },
            ],
            defaults,
            examples: vec![],
            tags: vec!["raster".to_string(), "spatial-regression".to_string(), "sar".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Err(ToolError::Validation(
            "spatial_lag_regression_raster is not yet implemented".to_string(),
        ))
    }

    fn run(&self, _args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Err(ToolError::Execution(
            "spatial_lag_regression_raster is not yet implemented".to_string(),
        ))
    }
}

impl Tool for SpatialErrorRegressionRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spatial_error_regression_raster",
            display_name: "Spatial Error Regression (SEM) - Raster Output",
            summary: "[NOT YET IMPLEMENTED] Outputs fitted value surface from spatial error regression.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor fields.", required: true },
                ToolParamSpec { name: "output", description: "Output raster (fitted values).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1"));
        defaults.insert("output".to_string(), json!("fitted.tif"));

        ToolManifest {
            id: "spatial_error_regression_raster".to_string(),
            display_name: "Spatial Error Regression (SEM) - Raster Output".to_string(),
            summary: "[NOT YET IMPLEMENTED] Outputs fitted value surface from SEM model.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output raster.".to_string(), required: true },
            ],
            defaults,
            examples: vec![],
            tags: vec!["raster".to_string(), "spatial-regression".to_string(), "sem".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Err(ToolError::Validation(
            "spatial_error_regression_raster is not yet implemented".to_string(),
        ))
    }

    fn run(&self, _args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Err(ToolError::Execution(
            "spatial_error_regression_raster is not yet implemented".to_string(),
        ))
    }
}

impl Tool for GeographicallyWeightedRegressionRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "geographically_weighted_regression_raster",
            display_name: "Geographically Weighted Regression (GWR) - Raster Output",
            summary: "[NOT YET IMPLEMENTED] Outputs local coefficient rasters from GWR model.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor fields.", required: true },
                ToolParamSpec { name: "output", description: "Output raster prefix (multiple bands for coefficients).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1"));
        defaults.insert("output".to_string(), json!("gwr_coef.tif"));

        ToolManifest {
            id: "geographically_weighted_regression_raster".to_string(),
            display_name: "Geographically Weighted Regression (GWR) - Raster Output".to_string(),
            summary: "[NOT YET IMPLEMENTED] Outputs local coefficient surfaces from GWR.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output raster.".to_string(), required: true },
            ],
            defaults,
            examples: vec![],
            tags: vec!["raster".to_string(), "spatial-regression".to_string(), "gwr".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, _args: &ToolArgs) -> Result<(), ToolError> {
        Err(ToolError::Validation(
            "geographically_weighted_regression_raster is not yet implemented".to_string(),
        ))
    }

    fn run(&self, _args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Err(ToolError::Execution(
            "geographically_weighted_regression_raster is not yet implemented".to_string(),
        ))
    }
}
