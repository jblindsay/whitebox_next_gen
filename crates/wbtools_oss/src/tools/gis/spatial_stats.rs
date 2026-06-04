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
pub struct InhomogeneousIntensityTool;
pub struct RipleysKTool;
pub struct EnvelopeTestTool;
pub struct PointProcessResidualsTool;

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

fn parse_optional_string_arg(args: &ToolArgs, key: &str) -> Result<Option<String>, ToolError> {
    match args.get(key) {
        None => Ok(None),
        Some(value) => {
            let Some(s) = value.as_str() else {
                return Err(ToolError::Validation(format!(
                    "Expected string for field '{}', got '{}'",
                    key, value
                )));
            };
            Ok(Some(s.to_string()))
        }
    }
}

fn parse_vector_path_arg(args: &ToolArgs, key: &str) -> Result<String, ToolError> {
    super::parse_string_arg(args, key).map(|s| s.to_string())
}

#[allow(dead_code)]
fn parse_raster_path_arg(args: &ToolArgs, key: &str) -> Result<String, ToolError> {
    super::parse_string_arg(args, key).map(|s| s.to_string())
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
            summary: r#"Computes Global Moran's I, a fundamental test of spatial autocorrelation measuring whether similar values cluster together spatially. Moran's I ranges from -1 (perfect negative autocorrelation/dispersed) to +1 (perfect positive autocorrelation/clustered), with 0 indicating random spatial arrangement. Significant positive I indicates clustering; negative I indicates values alternate among neighbors.

The test reveals whether your phenomenon is truly spatially dependent (guides kriging appropriateness, geostatistical method selection). Global Moran's I summarizes entire study area in one index. Both parametric (asymptotic) and permutation-based significance testing are supported. Permutation testing is more robust and recommended for non-normal data or small samples.

Outputs: I statistic, expected I under null hypothesis, variance, z-score, p-value, and interpretation. Optional permutation distribution visualization. Significant global autocorrelation suggests spatial non-stationarity—consider local LISA analysis to identify specific clusters. Use weights parameters (queen/rook/k-nearest/distance) to define neighborhoods."#,
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field to analyze.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode.", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "inference", description: "Inference mode: asymptotic or permutation (default: asymptotic).", required: false },
                ToolParamSpec { name: "num_simulations", description: "Number of permutations for permutation testing (default: 999).", required: false },
                ToolParamSpec { name: "seed", description: "Random seed for reproducible permutation testing (default: u64::MAX).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "output_json", description: "Optional JSON report output path.", required: false },
                ToolParamSpec { name: "output_html", description: "Optional HTML report output path.", required: false },
                ToolParamSpec { name: "output_csv", description: "Optional CSV summary output path.", required: false },
                ToolParamSpec { name: "output_distribution", description: "Optional path to save permutation distribution as JSON (permutation mode only).", required: false },
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
        defaults.insert("num_simulations".to_string(), json!(999));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));

        let mut example_args = defaults.clone();
        example_args.insert("output_json".to_string(), json!("morans_i_report.json"));

        let mut permutation_args = defaults.clone();
        permutation_args.insert("inference".to_string(), json!("permutation"));
        permutation_args.insert("num_simulations".to_string(), json!(999));

        ToolManifest {
            id: "global_morans_i".to_string(),
            display_name: "Global Moran's I".to_string(),
            summary: r#"Computes Global Moran's I to test spatial autocorrelation: whether similar values cluster spatially. Essential foundation for geostatistical analysis."#.to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field to analyze.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode: queen, rook, k_nearest, distance_band.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest mode.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold for distance_band mode.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Apply row standardization to weights (default true).".to_string(), required: false },
                ToolParamDescriptor { name: "inference".to_string(), description: "Inference mode: asymptotic or permutation (default: asymptotic).".to_string(), required: false },
                ToolParamDescriptor { name: "num_simulations".to_string(), description: "Number of permutations for permutation testing (default: 999).".to_string(), required: false },
                ToolParamDescriptor { name: "seed".to_string(), description: "Random seed for reproducible permutation testing.".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "output_json".to_string(), description: "Optional JSON report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_csv".to_string(), description: "Optional CSV summary output path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_distribution".to_string(), description: "Optional path to save permutation distribution as JSON.".to_string(), required: false },
            ],
            defaults,
            examples: vec![
                ToolExample {
                    name: "global_morans_i_asymptotic".to_string(),
                    description: "Computes Global Moran's I with asymptotic inference.".to_string(),
                    args: example_args,
                },
                ToolExample {
                    name: "global_morans_i_permutation".to_string(),
                    description: "Computes Global Moran's I with permutation-based inference (999 simulations).".to_string(),
                    args: permutation_args,
                },
            ],
            tags: vec![
                "vector".to_string(),
                "spatial-statistics".to_string(),
                "autocorrelation".to_string(),
                "permutation-testing".to_string(),
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

        // Validate permutation parameters
        if inference == "permutation" {
            let n_sims = parse_optional_usize_arg(args, "num_simulations")?.unwrap_or(999);
            if n_sims < 10 || n_sims > 1000000 {
                return Err(ToolError::Validation(
                    "num_simulations must be between 10 and 1,000,000".to_string(),
                ));
            }
        }

        let _ = IslandPolicy::parse(args)?;
        let _ = parse_optional_output_path(args, "output_json")?;
        let _ = parse_optional_output_path(args, "output_html")?;
        let _ = parse_optional_output_path(args, "output_csv")?;
        let _ = parse_optional_output_path(args, "output_distribution")?;
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
        let num_simulations = parse_optional_usize_arg(args, "num_simulations")?.unwrap_or(999);
        let seed = parse_optional_usize_arg(args, "seed").map(|opt| opt.map(|s| s as u64)).ok().flatten();

        let output_json = parse_optional_output_path(args, "output_json")?;
        let output_html = parse_optional_output_path(args, "output_html")?;
        let output_csv = parse_optional_output_path(args, "output_csv")?;
        let output_distribution = parse_optional_output_path(args, "output_distribution")?;

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

        // Determine inference method and compute results
        let (final_p_value, permutation_distribution, inference_method) = if inference == "permutation" {
            ctx.progress.info(&format!("computing permutation test ({} simulations)", num_simulations));
            let perm_result = wbspatialstats::autocorrelation::permutation::morans_i_permutation(
                &values,
                &weights,
                num_simulations,
                seed,
            ).map_err(|e| ToolError::Execution(format!("permutation test failed: {}", e)))?;

            (perm_result.p_value_two_tailed, Some(perm_result.permutation_distribution), "permutation")
        } else {
            (p_value, None, "asymptotic")
        };

        let mut report = serde_json::Map::new();
        report.insert("tool_id".to_string(), json!("global_morans_i"));
        report.insert("inference_method".to_string(), json!(inference_method));
        report.insert("statistic_i".to_string(), json!(statistic_i));
        report.insert("expected_i".to_string(), json!(expected_i));
        report.insert(
            "variance_i".to_string(),
            json!(((statistic_i - expected_i) / z_score).powi(2)),
        );
        report.insert("z_score".to_string(), json!(z_score));
        report.insert("p_value_asymptotic".to_string(), json!(p_value));
        report.insert("p_value_two_sided".to_string(), json!(final_p_value));
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
        let significance_class = if final_p_value <= 0.05 && statistic_i > expected_i {
            "positive"
        } else if final_p_value <= 0.05 && statistic_i < expected_i {
            "negative"
        } else {
            "ns"
        };
        report.insert("significance_class".to_string(), json!(significance_class));
        
        // Add permutation-specific info if applicable
        if inference == "permutation" {
            if let Some(dist) = &permutation_distribution {
                report.insert("permutation_distribution_size".to_string(), json!(dist.len()));
                report.insert("permutation_distribution_mean".to_string(), 
                    json!(dist.iter().sum::<f64>() / dist.len() as f64));
                report.insert("permutation_distribution_min".to_string(),
                    json!(dist.iter().cloned().fold(f64::INFINITY, f64::min)));
                report.insert("permutation_distribution_max".to_string(),
                    json!(dist.iter().cloned().fold(f64::NEG_INFINITY, f64::max)));
            }
        }
        
        report.insert(
            "assumption_flags".to_string(),
            json!({
                "permutation_supported": true,
                "inference": inference_method,
            }),
        );
        report.insert(
            "runtime_metadata".to_string(),
            json!({
                "seed": seed,
                "permutations": if inference == "permutation" { Some(num_simulations) } else { None },
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

        if let Some(path) = output_distribution {
            if let Some(dist) = &permutation_distribution {
                let dist_json = json!({
                    "observed_statistic": statistic_i,
                    "p_value_two_tailed": final_p_value,
                    "permutation_distribution": dist,
                    "n_simulations": dist.len(),
                });
                let body = serde_json::to_string_pretty(&dist_json)
                    .map_err(|e| ToolError::Execution(format!("failed serializing distribution: {e}")))?;
                write_text(&path, &body)?;
                outputs.insert("output_distribution".to_string(), json!(path.to_string_lossy().to_string()));
            } else {
                ctx.progress.info("WARNING: output_distribution specified but inference mode is asymptotic (not permutation)");
            }
        }

        if let Some(path) = output_csv {
            let p_text = final_p_value.to_string();
            let body = if inference == "permutation" {
                format!(
                    "tool_id,statistic_i,expected_i,z_score,p_value_asymptotic,p_value_permutation,n_simulations,n_features_used,n_features_dropped\nglobal_morans_i,{},{},{},{},{},{},{},{}\n",
                    statistic_i,
                    expected_i,
                    z_score,
                    p_value,
                    p_text,
                    num_simulations,
                    n_used,
                    weights.diagnostics.dropped_feature_count,
                )
            } else {
                format!(
                    "tool_id,statistic_i,expected_i,z_score,p_value_asymptotic,n_features_used,n_features_dropped\nglobal_morans_i,{},{},{},{},{},{}\n",
                    statistic_i,
                    expected_i,
                    z_score,
                    p_value,
                    n_used,
                    weights.diagnostics.dropped_feature_count,
                )
            };
            write_text(&path, &body)?;
            outputs.insert("output_csv".to_string(), json!(path.to_string_lossy().to_string()));
        }

        if let Some(path) = output_html {
            let z_text = format!("{z_score:.6}");
            let p_text = format!("{final_p_value:.6}");
            let body = if inference == "permutation" {
                build_branded_html_report(
                    "Global Moran's I Report (Permutation Testing)",
                    &[
                        "Statistic I",
                        "Expected I",
                        "Z (asymptotic)",
                        "P (permutation)",
                        "N simulations",
                        "N used",
                        "N dropped",
                    ],
                    &[
                        format!("{statistic_i:.6}"),
                        format!("{expected_i:.6}"),
                        z_text,
                        p_text,
                        num_simulations.to_string(),
                        n_used.to_string(),
                        weights.diagnostics.dropped_feature_count.to_string(),
                    ],
                )
            } else {
                build_branded_html_report(
                    "Global Moran's I Report (Asymptotic Testing)",
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
                )
            };
            write_text(&path, &body)?;
            outputs.insert("output_html".to_string(), json!(path.to_string_lossy().to_string()));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

#[derive(Clone, Copy)]
enum MultipleTestingMode {
    #[allow(dead_code)]
    None,
    FdrBh,
    #[allow(dead_code)]
    Bonferroni,
}

impl MultipleTestingMode {
    #[allow(dead_code)]
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
            summary: r#"Computes Local Moran's I (LISA) for each feature, identifying statistically significant local spatial clusters and outliers. While Global Moran's I summarizes entire study area, LISA reveals which locations contribute most to global clustering and classifies them into cluster types: HH (high-value clusters), LL (low-value clusters), HL (high outliers), LH (low outliers).

LISA results pinpoint hotspots and coldspots, allowing targeted analysis of cluster drivers. Output includes LISA values, p-values, cluster classification, and optional FDR multiple-testing correction to control false discovery. Both asymptotic and permutation inference are supported; permutation-based p-values are more reliable for non-normal data.

Applications: identifying crime hotspots, disease clusters, pollution zones, or high-value/low-value neighborhoods. Map LISA cluster classifications to visualize spatial structure. Investigate cluster drivers by examining feature attributes within identified clusters. Use weights parameters to define local neighborhoods (k-nearest recommended; vary k for sensitivity analysis)."#,
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field to analyze.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode.", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "inference", description: "Inference mode: asymptotic or permutation (default: asymptotic).", required: false },
                ToolParamSpec { name: "num_simulations", description: "Number of permutations for permutation testing (default: 999).", required: false },
                ToolParamSpec { name: "seed", description: "Random seed for reproducible permutation testing (default: u64::MAX).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "alpha", description: "Significance threshold in [0, 1]; default 0.05.", required: false },
                ToolParamSpec { name: "fdr_correction", description: "Apply FDR-BH correction (default true for permutation).", required: false },
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
        defaults.insert("num_simulations".to_string(), json!(999));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));
        defaults.insert("alpha".to_string(), json!(0.05));
        defaults.insert("fdr_correction".to_string(), json!(true));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("lisa_output.gpkg"));

        let mut permutation_args = defaults.clone();
        permutation_args.insert("inference".to_string(), json!("permutation"));
        permutation_args.insert("output".to_string(), json!("lisa_output_perm.gpkg"));

        ToolManifest {
            id: "local_morans_i_lisa".to_string(),
            display_name: "Local Moran's I (LISA)".to_string(),
            summary: r#"Computes Local Moran's I for each feature to identify local spatial clusters and outliers with statistical significance testing."#.to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field to analyze.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode: queen, rook, k_nearest, distance_band.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k value for k_nearest mode.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold for distance_band mode.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Apply row standardization to weights (default true).".to_string(), required: false },
                ToolParamDescriptor { name: "inference".to_string(), description: "Inference mode: asymptotic or permutation (default: asymptotic).".to_string(), required: false },
                ToolParamDescriptor { name: "num_simulations".to_string(), description: "Number of permutations for permutation testing (default: 999).".to_string(), required: false },
                ToolParamDescriptor { name: "seed".to_string(), description: "Random seed for reproducible permutation testing.".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "alpha".to_string(), description: "Significance threshold in [0, 1]; default 0.05.".to_string(), required: false },
                ToolParamDescriptor { name: "fdr_correction".to_string(), description: "Apply FDR-BH correction (default true for permutation).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path with LISA fields.".to_string(), required: true },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![
                ToolExample {
                    name: "local_morans_i_lisa_asymptotic".to_string(),
                    description: "Computes Local Moran's I with asymptotic inference.".to_string(),
                    args: example_args,
                },
                ToolExample {
                    name: "local_morans_i_lisa_permutation".to_string(),
                    description: "Computes Local Moran's I with permutation-based inference (999 simulations).".to_string(),
                    args: permutation_args,
                },
            ],
            tags: vec![
                "vector".to_string(),
                "spatial-statistics".to_string(),
                "autocorrelation".to_string(),
                "lisa".to_string(),
                "permutation-testing".to_string(),
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

        // Validate permutation parameters
        if inference == "permutation" {
            let n_sims = parse_optional_usize_arg(args, "num_simulations")?.unwrap_or(999);
            if n_sims < 10 || n_sims > 1000000 {
                return Err(ToolError::Validation(
                    "num_simulations must be between 10 and 1,000,000".to_string(),
                ));
            }
        }

        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(ToolError::Validation("alpha must be in [0, 1]".to_string()));
        }

        let _ = IslandPolicy::parse(args)?;
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
        let num_simulations = parse_optional_usize_arg(args, "num_simulations")?.unwrap_or(999);
        let seed = parse_optional_usize_arg(args, "seed").map(|opt| opt.map(|s| s as u64)).ok().flatten();
        let fdr_correction = parse_bool_arg(args, "fdr_correction", inference == "permutation");
        let output_path = parse_vector_path_arg(args, "output")?;
        let output_html = parse_optional_output_path(args, "output_html")?;

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
        
        let (lisa_i, lisa_z, lisa_p, quadrant, inference_method) = if inference == "permutation" {
            ctx.progress.info(&format!("computing permutation test ({} simulations)", num_simulations));
            let perm_result = wbspatialstats::autocorrelation::permutation::local_morans_i_permutation(
                &values,
                &weights,
                num_simulations,
                fdr_correction,
                seed,
            ).map_err(|e| ToolError::Execution(format!("permutation test failed: {}", e)))?;
            
            let lisa_i_vec: Vec<Option<f64>> = perm_result.observed_statistics.iter().map(|&s| Some(s)).collect();
            let lisa_z_vec: Vec<Option<f64>> = perm_result.z_scores.iter().map(|&s| Some(s)).collect();
            let lisa_p_vec: Vec<Option<f64>> = perm_result.p_values.iter().map(|&s| Some(s)).collect();
            let quadrant_vec = perm_result.cluster_types.clone();
            
            (
                lisa_i_vec,
                lisa_z_vec,
                lisa_p_vec,
                quadrant_vec,
                "permutation"
            )
        } else {
            let (i, z, p, q) = compute_local_morans_i_lisa(&values, &weights, island_policy, alpha)?;
            (i, z, p, q, "asymptotic")
        };

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

        // P-values from permutation test are already adjusted if fdr_correction=true
        // For asymptotic test, lisa_p is unadjusted; we use it as-is
        let lisa_p_adj = &lisa_p;

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

        let locator = write_vector_output(&output, output_path.as_str())?;

        let n_features_used = n_obs - weights.diagnostics.dropped_feature_count - island_count;

        let summary = json!({
                "tool_id": "local_morans_i_lisa",
                "inference_method": inference_method,
                "statistic": serde_json::Value::Null,
                "p_value": serde_json::Value::Null,
                "alpha": alpha,
                "significance_class": serde_json::Value::Null,
                "fdr_correction": fdr_correction,
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
                    "permutation_supported": true,
                    "inference": inference_method,
                },
                "runtime_metadata": {
                    "seed": seed,
                    "permutations": if inference == "permutation" { Some(num_simulations) } else { None },
                },
            });

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), json!(locator));
        outputs.insert("summary".to_string(), summary.clone());
        outputs.insert("report".to_string(), summary);

        if let Some(path) = output_html {
            let body = if inference == "permutation" {
                build_branded_html_report(
                    "Local Moran's I (LISA) Report (Permutation Testing)",
                    &[
                        "HH",
                        "LL",
                        "HL",
                        "LH",
                        "NS",
                        "N used",
                        "N dropped",
                        "N islands",
                        "N simulations",
                        "alpha",
                        "FDR correction",
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
                        num_simulations.to_string(),
                        format!("{alpha:.6}"),
                        if fdr_correction { "true" } else { "false" }.to_string(),
                    ],
                )
            } else {
                build_branded_html_report(
                    "Local Moran's I (LISA) Report (Asymptotic Testing)",
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
                        "FDR correction",
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
                        if fdr_correction { "true" } else { "false" }.to_string(),
                    ],
                )
            };
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
            summary: r#"Computes Getis-Ord Gi (or Gi* variant) z-scores measuring local concentration of high or low values, with integrated hotspot/coldspot classification. Unlike Moran's I which compares neighbors to global mean, Gi* compares each location's neighborhood sum to study-area average, directly identifying statistically significant hotspots (high-value clusters with z > 0) and coldspots (low-value clusters with z < 0).

Gi* variant (recommended default) includes the feature itself in neighborhood sum, providing stronger signal for isolated clusters. Gi excludes self and emphasizes surrounding context. Both variants output z-scores, p-values (asymptotic or permutation-based), and cluster classification (hotspot/coldspot/insignificant at chosen alpha level).

Applications: identifying retail sales hotspots, disease/crime hotspots, pollution concentration zones, poverty concentration areas. More interpretable than Moran's I for practitioners—positive z-scores directly indicate high-value concentration. Use weights parameters to define neighborhoods; k-nearest recommended. Permutation testing preferable for non-normal or small sample data. Map z-scores for continuous visualization; classifications for categorical interpretation."#,
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
                ToolParamSpec { name: "inference", description: "Inference mode: asymptotic or permutation (default: asymptotic).", required: false },
                ToolParamSpec { name: "num_simulations", description: "Number of permutations for permutation testing (default: 999).", required: false },
                ToolParamSpec { name: "seed", description: "Random seed for reproducible permutation testing (default: u64::MAX).", required: false },
                ToolParamSpec { name: "island_policy", description: "Island handling: drop_with_warning, keep_zero_weight, error.", required: false },
                ToolParamSpec { name: "alpha", description: "Significance threshold in [0, 1]; default 0.05.", required: false },
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
        defaults.insert("num_simulations".to_string(), json!(999));
        defaults.insert("island_policy".to_string(), json!("drop_with_warning"));
        defaults.insert("alpha".to_string(), json!(0.05));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("gi_star_output.gpkg"));

        let mut permutation_args = defaults.clone();
        permutation_args.insert("inference".to_string(), json!("permutation"));
        permutation_args.insert("output".to_string(), json!("gi_star_output_perm.gpkg"));

        ToolManifest {
            id: "getis_ord_gi_star".to_string(),
            display_name: "Getis-Ord Gi / Gi*".to_string(),
            summary: r#"Computes Getis-Ord Gi/Gi* z-scores for local hotspot/coldspot identification. Direct measure of high/low value concentration."#.to_string(),
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
                ToolParamDescriptor { name: "inference".to_string(), description: "Inference mode: asymptotic or permutation (default: asymptotic).".to_string(), required: false },
                ToolParamDescriptor { name: "num_simulations".to_string(), description: "Number of permutations for permutation testing (default: 999).".to_string(), required: false },
                ToolParamDescriptor { name: "seed".to_string(), description: "Random seed for reproducible permutation testing.".to_string(), required: false },
                ToolParamDescriptor { name: "island_policy".to_string(), description: "Island handling: drop_with_warning, keep_zero_weight, error.".to_string(), required: false },
                ToolParamDescriptor { name: "alpha".to_string(), description: "Significance threshold in [0, 1]; default 0.05.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path with GI fields.".to_string(), required: true },
                ToolParamDescriptor { name: "output_html".to_string(), description: "Optional HTML report output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![
                ToolExample {
                    name: "getis_ord_gi_star_asymptotic".to_string(),
                    description: "Computes Gi* with asymptotic inference.".to_string(),
                    args: example_args,
                },
                ToolExample {
                    name: "getis_ord_gi_star_permutation".to_string(),
                    description: "Computes Gi* with permutation-based inference (999 simulations).".to_string(),
                    args: permutation_args,
                },
            ],
            tags: vec![
                "vector".to_string(),
                "spatial-statistics".to_string(),
                "hotspot".to_string(),
                "coldspot".to_string(),
                "permutation-testing".to_string(),
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

        // Validate permutation parameters
        if inference == "permutation" {
            let n_sims = parse_optional_usize_arg(args, "num_simulations")?.unwrap_or(999);
            if n_sims < 10 || n_sims > 1000000 {
                return Err(ToolError::Validation(
                    "num_simulations must be between 10 and 1,000,000".to_string(),
                ));
            }
        }

        let alpha = parse_optional_f64_arg(args, "alpha").unwrap_or(0.05);
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(ToolError::Validation("alpha must be in [0, 1]".to_string()));
        }

        let _ = IslandPolicy::parse(args)?;
        let _ = GiVariant::parse(args)?;
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
        let num_simulations = parse_optional_usize_arg(args, "num_simulations")?.unwrap_or(999);
        let seed = parse_optional_usize_arg(args, "seed").map(|opt| opt.map(|s| s as u64)).ok().flatten();
        let output_path = parse_vector_path_arg(args, "output")?;
        let output_html = parse_optional_output_path(args, "output_html")?;

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
        let (gi_z, gi_p, cluster_type, inference_method) = if inference == "permutation" {
            ctx.progress.info(&format!("computing permutation test ({} simulations)", num_simulations));
            let perm_result = wbspatialstats::autocorrelation::permutation::getis_ord_gi_star_permutation(
                &values,
                &weights,
                num_simulations,
                seed,
            ).map_err(|e| ToolError::Execution(format!("permutation test failed: {}", e)))?;
            
            let gi_z_vec: Vec<Option<f64>> = vec![Some(perm_result.z_score); values.len()];
            let gi_p_vec: Vec<Option<f64>> = vec![Some(perm_result.p_value_two_tailed); values.len()];
            let cluster_vec: Vec<String> = values.iter().enumerate().map(|(_i, &v)| {
                let z = perm_result.z_score;
                if v > values.iter().sum::<f64>() / values.len() as f64 {
                    if z > 1.96 { "HotSpot".to_string() } else { "NotSignificant".to_string() }
                } else {
                    if z < -1.96 { "ColdSpot".to_string() } else { "NotSignificant".to_string() }
                }
            }).collect();
            
            (gi_z_vec, gi_p_vec, cluster_vec, "permutation")
        } else {
            let (z, p, c) = compute_getis_ord_gi_star(&values, &weights, island_policy, alpha)?;
            (z, p, c, "asymptotic")
        };

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

        // For permutation test, p-values are already global
        // For asymptotic, apply multiple testing correction if needed
        let gi_p_adj = if inference == "asymptotic" {
            let multiple_testing = MultipleTestingMode::FdrBh;
            adjust_p_values(&gi_p, multiple_testing)
        } else {
            gi_p.clone()
        };

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

        let locator = write_vector_output(&output, output_path.as_str())?;

        let n_features_used = n_obs - weights.diagnostics.dropped_feature_count - island_count;

        let summary = json!({
                "tool_id": "getis_ord_gi_star",
                "inference_method": inference_method,
                "variant": match variant {
                    GiVariant::Gi => "gi",
                    GiVariant::GiStar => "gi_star",
                },
                "statistic": serde_json::Value::Null,
                "p_value": serde_json::Value::Null,
                "alpha": alpha,
                "significance_class": serde_json::Value::Null,
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
                    "permutation_supported": true,
                    "inference": inference_method,
                },
                "runtime_metadata": {
                    "seed": seed,
                    "permutations": if inference == "permutation" { Some(num_simulations) } else { None },
                },
            });

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), json!(locator));
        outputs.insert("summary".to_string(), summary.clone());
        outputs.insert("report".to_string(), summary);

        if let Some(path) = output_html {
            let body = if inference == "permutation" {
                build_branded_html_report(
                    "Getis-Ord Gi / Gi* Report (Permutation Testing)",
                    &[
                        "hot",
                        "cold",
                        "ns",
                        "N used",
                        "N dropped",
                        "N islands",
                        "N simulations",
                        "alpha",
                        "variant",
                    ],
                    &[
                        hot.to_string(),
                        cold.to_string(),
                        ns.to_string(),
                        n_features_used.to_string(),
                        weights.diagnostics.dropped_feature_count.to_string(),
                        island_count.to_string(),
                        num_simulations.to_string(),
                        format!("{alpha:.6}"),
                        match variant {
                            GiVariant::Gi => "gi".to_string(),
                            GiVariant::GiStar => "gi_star".to_string(),
                        },
                    ],
                )
            } else {
                build_branded_html_report(
                    "Getis-Ord Gi / Gi* Report (Asymptotic Testing)",
                    &[
                        "hot",
                        "cold",
                        "ns",
                        "N used",
                        "N dropped",
                        "N islands",
                        "alpha",
                        "variant",
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
                    ],
                )
            };
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

        let locator = write_vector_output(&output_layer, output_path.as_str())?;

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

        let locator = write_vector_output(&output_layer, output_path.as_str())?;

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

        let locator = write_vector_output(&output_layer, output_path.as_str())?;

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
        let _ = parse_optional_output_path(args, "output")?;
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
        let output_path = parse_optional_output_path(args, "output")?;

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
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;

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
        let _ = parse_vector_path_arg(args, "output")?;
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
        let output_path = parse_optional_output_path(args, "output")?;

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
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// PHASE C RASTER TOOLS (Spatial Regression - Fitted Value Surfaces)
// ============================================================================

impl Tool for SpatialLagRegressionRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spatial_lag_regression_raster",
            display_name: "Spatial Lag Regression (SAR) - Raster Output",
            summary: "Estimates spatial lag regression and outputs fitted value surface.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor fields.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode (default 8).", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "cell_size", description: "Output raster cell size (optional; uses input extent).", required: false },
                ToolParamSpec { name: "output", description: "Output raster (fitted values).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1"));
        defaults.insert("weights_mode".to_string(), json!("k_nearest"));
        defaults.insert("k".to_string(), json!(8));
        defaults.insert("row_standardize".to_string(), json!(true));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("fitted.tif"));

        ToolManifest {
            id: "spatial_lag_regression_raster".to_string(),
            display_name: "Spatial Lag Regression (SAR) - Raster Output".to_string(),
            summary: "Estimates SAR model and outputs fitted value surface.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k for k_nearest.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Row standardize weights.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output raster cell size.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output raster.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "sar_raster_basic".to_string(),
                description: "Estimate SAR and interpolate fitted values to raster.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "spatial-regression".to_string(), "sar".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "response_field")?;
        let _ = parse_string_arg(args, "predictor_fields")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        use wbspatialstats::regression::SpatialLagRegression;
        use nalgebra::DMatrix;

        let input = load_vector_arg(args, "input")?;
        let response_field = parse_string_arg(args, "response_field")?;
        let predictor_str = parse_string_arg(args, "predictor_fields")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let island_policy = IslandPolicy::parse(args)?;
        let cell_size = parse_optional_f64_arg(args, "cell_size");

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
                    pred_field, pred_obs.len(), n
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

        ctx.progress.info("Building output raster");
        let samples: Vec<(f64, f64, f64)> = observations.iter().map(|o| (o.x, o.y, o.value)).collect();
        let mut output = super::build_point_interpolation_output(&input, &samples, cell_size, None, DataType::F64)?;

        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;

        ctx.progress.info("Interpolating fitted values to raster grid");
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

                // Use fitted value from nearest observation
                let fitted_value = result.base.fitted[nearest_idx];
                let idx = row * cols + col;
                output.data.set_f64(idx, fitted_value);
            }

            let progress = (row as f64 + 1.0) / rows as f64;
            ctx.progress.progress(progress);
        }

        ctx.progress.info("Writing raster output");
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for SpatialErrorRegressionRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spatial_error_regression_raster",
            display_name: "Spatial Error Regression (SEM) - Raster Output",
            summary: "Estimates spatial error regression and outputs fitted value surface.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor fields.", required: true },
                ToolParamSpec { name: "weights_mode", description: "Neighborhood mode: queen, rook, k_nearest, distance_band.", required: false },
                ToolParamSpec { name: "k", description: "k value for k_nearest mode (default 8).", required: false },
                ToolParamSpec { name: "distance", description: "Distance threshold for distance_band mode.", required: false },
                ToolParamSpec { name: "row_standardize", description: "Apply row standardization to weights (default true).", required: false },
                ToolParamSpec { name: "cell_size", description: "Output raster cell size (optional; uses input extent).", required: false },
                ToolParamSpec { name: "output", description: "Output raster (fitted values).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1"));
        defaults.insert("weights_mode".to_string(), json!("k_nearest"));
        defaults.insert("k".to_string(), json!(8));
        defaults.insert("row_standardize".to_string(), json!(true));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("fitted.tif"));

        ToolManifest {
            id: "spatial_error_regression_raster".to_string(),
            display_name: "Spatial Error Regression (SEM) - Raster Output".to_string(),
            summary: "Estimates SEM model and outputs fitted value surface.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "weights_mode".to_string(), description: "Neighborhood mode.".to_string(), required: false },
                ToolParamDescriptor { name: "k".to_string(), description: "k for k_nearest.".to_string(), required: false },
                ToolParamDescriptor { name: "distance".to_string(), description: "Distance threshold.".to_string(), required: false },
                ToolParamDescriptor { name: "row_standardize".to_string(), description: "Row standardize weights.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output raster cell size.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output raster.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "sem_raster_basic".to_string(),
                description: "Estimate SEM and interpolate fitted values to raster.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "spatial-regression".to_string(), "sem".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "response_field")?;
        let _ = parse_string_arg(args, "predictor_fields")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        use wbspatialstats::regression::SpatialErrorRegression;
        use nalgebra::DMatrix;

        let input = load_vector_arg(args, "input")?;
        let response_field = parse_string_arg(args, "response_field")?;
        let predictor_str = parse_string_arg(args, "predictor_fields")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mode = SpatialWeightsMode::parse(args)?;
        let k = parse_optional_usize_arg(args, "k")?.unwrap_or(8);
        let distance = parse_optional_f64_arg(args, "distance").unwrap_or(0.0);
        let row_standardize = parse_bool_arg(args, "row_standardize", true);
        let island_policy = IslandPolicy::parse(args)?;
        let cell_size = parse_optional_f64_arg(args, "cell_size");

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
                    pred_field, pred_obs.len(), n
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

        ctx.progress.info("Building output raster");
        let samples: Vec<(f64, f64, f64)> = observations.iter().map(|o| (o.x, o.y, o.value)).collect();
        let mut output = super::build_point_interpolation_output(&input, &samples, cell_size, None, DataType::F64)?;

        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;

        ctx.progress.info("Interpolating fitted values to raster grid");
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

                // Use fitted value from nearest observation
                let fitted_value = result.base.fitted[nearest_idx];
                let idx = row * cols + col;
                output.data.set_f64(idx, fitted_value);
            }

            let progress = (row as f64 + 1.0) / rows as f64;
            ctx.progress.progress(progress);
        }

        ctx.progress.info("Writing raster output");
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for GeographicallyWeightedRegressionRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "geographically_weighted_regression_raster",
            display_name: "Geographically Weighted Regression (GWR) - Raster Output",
            summary: "Estimates GWR and outputs local coefficient raster surfaces.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "response_field", description: "Response variable.", required: true },
                ToolParamSpec { name: "predictor_fields", description: "Comma-separated predictor fields.", required: true },
                ToolParamSpec { name: "bandwidth", description: "Bandwidth for kernel (default: adaptive).", required: false },
                ToolParamSpec { name: "kernel", description: "Kernel type: gaussian, bisquare (default: bisquare).", required: false },
                ToolParamSpec { name: "cell_size", description: "Output raster cell size (optional; uses input extent).", required: false },
                ToolParamSpec { name: "output_prefix", description: "Output raster filename prefix (adds _coef_X suffix per predictor).", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("response_field".to_string(), json!("response"));
        defaults.insert("predictor_fields".to_string(), json!("predictor1"));
        defaults.insert("kernel".to_string(), json!("bisquare"));

        let mut example_args = defaults.clone();
        example_args.insert("output_prefix".to_string(), json!("gwr_coef.tif"));

        ToolManifest {
            id: "geographically_weighted_regression_raster".to_string(),
            display_name: "Geographically Weighted Regression (GWR) - Raster Output".to_string(),
            summary: "Estimates GWR and outputs local coefficient raster surfaces.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "response_field".to_string(), description: "Response variable.".to_string(), required: true },
                ToolParamDescriptor { name: "predictor_fields".to_string(), description: "Predictor fields.".to_string(), required: true },
                ToolParamDescriptor { name: "bandwidth".to_string(), description: "Kernel bandwidth.".to_string(), required: false },
                ToolParamDescriptor { name: "kernel".to_string(), description: "Kernel type.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output raster cell size.".to_string(), required: false },
                ToolParamDescriptor { name: "output_prefix".to_string(), description: "Output raster prefix.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "gwr_raster_basic".to_string(),
                description: "Estimate GWR and output local coefficient surfaces.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "spatial-regression".to_string(), "gwr".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_string_arg(args, "response_field")?;
        let _ = parse_string_arg(args, "predictor_fields")?;
        let _ = parse_optional_output_path(args, "output_prefix")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        use wbspatialstats::regression::GeographicallyWeightedRegression;
        use nalgebra::DMatrix;

        let input = load_vector_arg(args, "input")?;
        let response_field = parse_string_arg(args, "response_field")?;
        let predictor_str = parse_string_arg(args, "predictor_fields")?;
        let output_prefix = parse_optional_output_path(args, "output_prefix")?;
        let _kernel = parse_string_arg(args, "kernel").unwrap_or_else(|_| "bisquare");
        let cell_size = parse_optional_f64_arg(args, "cell_size");

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

        // Extract predictor fields
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
                    pred_field, pred_obs.len(), n
                )));
            }
            for obs in pred_obs {
                x_data.push(obs.value);
            }
        }

        let x = DMatrix::from_column_slice(n, 1 + predictor_fields.len(), &x_data);
        let coords: Vec<(f64, f64)> = observations.iter().map(|o| (o.x, o.y)).collect();

        ctx.progress.info("Estimating GWR model");
        let bandwidth_hint = parse_optional_f64_arg(args, "bandwidth");
        let result = GeographicallyWeightedRegression::estimate(&y, &x, &coords, bandwidth_hint)
            .map_err(|e| ToolError::Execution(format!("GWR estimation failed: {}", e)))?;

        ctx.progress.info("Building output rasters");
        let samples: Vec<(f64, f64, f64)> = observations.iter().map(|o| (o.x, o.y, o.value)).collect();
        let base_raster = super::build_point_interpolation_output(&input, &samples, cell_size, None, DataType::F64)?;

        // For each predictor + intercept, create a coefficient raster
        let mut outputs = ToolArgs::new();
        let n_coefs = 1 + predictor_fields.len();
        
        // Handle optional output prefix
        let prefix_str = output_prefix.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "gwr_coef".to_string());
        let prefix_base = prefix_str.trim_end_matches(".tif").trim_end_matches(".img").trim_end_matches(".hdf");

        for coef_idx in 0..n_coefs {
            let coef_label = if coef_idx == 0 {
                "intercept".to_string()
            } else {
                predictor_fields[coef_idx - 1].to_string()
            };

            let mut output = base_raster.clone();
            let rows = output.rows;
            let cols = output.cols;
            let x_min = output.x_min;
            let y_max = output.y_max();
            let cell_x = output.cell_size_x;
            let cell_y = output.cell_size_y;

            ctx.progress.info(&format!("Interpolating coefficient {} to raster grid", coef_label));
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

                    // Use local coefficient from nearest observation
                    let coef_value = result.local_coefficients[(nearest_idx, coef_idx)];
                    let idx = row * cols + col;
                    output.data.set_f64(idx, coef_value);
                }

                let progress = (row as f64 + 1.0) / rows as f64;
                ctx.progress.progress(progress);
            }

            // Write each coefficient raster
            let coef_output_path = if coef_idx == 0 {
                format!("{}_intercept.tif", prefix_base)
            } else {
                format!("{}_{}.tif", prefix_base, coef_label)
            };

            ctx.progress.info(&format!("Writing raster {}", coef_label));
            let locator = GisOverlayCore::store_or_write_output(output, Some(std::path::PathBuf::from(&coef_output_path)), ctx)?;
            outputs.insert(format!("coef_{}", coef_label), json!(locator));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

// ============================================================================
// PHASE D TOOLS (Point Process Analysis)
// ============================================================================

impl Tool for InhomogeneousIntensityTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "inhomogeneous_intensity_raster",
            display_name: "Inhomogeneous Intensity - Raster Output",
            summary: "Computes kernel density estimation surface for point pattern intensity.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "bandwidth", description: "Kernel bandwidth (default: auto-computed).", required: false },
                ToolParamSpec { name: "kernel", description: "Kernel type: gaussian, epanechnikov (default: gaussian).", required: false },
                ToolParamSpec { name: "cell_size", description: "Output raster cell size (optional).", required: false },
                ToolParamSpec { name: "output", description: "Output intensity raster.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("kernel".to_string(), json!("gaussian"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("intensity.tif"));

        ToolManifest {
            id: "inhomogeneous_intensity_raster".to_string(),
            display_name: "Inhomogeneous Intensity - Raster Output".to_string(),
            summary: "Computes kernel density estimation surface for point intensity analysis.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "bandwidth".to_string(), description: "Kernel bandwidth.".to_string(), required: false },
                ToolParamDescriptor { name: "kernel".to_string(), description: "Kernel type.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output raster cell size.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output intensity raster.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "kde_basic".to_string(),
                description: "Compute kernel density estimation surface.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "point-pattern".to_string(), "kde".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let kernel = parse_string_arg(args, "kernel").unwrap_or_else(|_| "gaussian");
        let cell_size = parse_optional_f64_arg(args, "cell_size");
        let bandwidth = parse_optional_f64_arg(args, "bandwidth");
        let output_path = parse_optional_output_path(args, "output")?;

        // Extract point coordinates
        let mut points = Vec::new();
        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    points.push((coord.x, coord.y));
                }
            }
        }

        if points.is_empty() {
            return Err(ToolError::Execution("No valid point features found".to_string()));
        }

        ctx.progress.info(&format!("Extracted {} points for KDE", points.len()));

        // Auto-compute bandwidth if not provided (Scott's rule)
        let h = if let Some(bw) = bandwidth {
            bw
        } else {
            // Scott's rule: h = n^(-1/3)
            let n = points.len() as f64;
            n.powf(-1.0 / 3.0) * 10.0 // Scale factor for geographic data
        };

        ctx.progress.info(&format!("Kernel bandwidth: {:.4}", h));

        // Build output raster with same extent as input layer
        let samples: Vec<(f64, f64, f64)> = points.iter().map(|(x, y)| (*x, *y, 1.0)).collect();
        let mut output = super::build_point_interpolation_output(&input, &samples, cell_size, None, DataType::F64)?;

        let rows = output.rows;
        let cols = output.cols;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;

        ctx.progress.info("Computing kernel density estimation");
        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                // Sum kernel contributions from all points
                let mut density = 0.0;
                for (px, py) in &points {
                    let dx = (x - px) / h;
                    let dy = (y - py) / h;
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt();

                    let k_val = if kernel == "bisquare" || kernel == "epanechnikov" {
                        if dist <= 1.0 {
                            (3.0 / 4.0) * (1.0 - dist * dist)
                        } else {
                            0.0
                        }
                    } else {
                        // Gaussian kernel (default)
                        (-0.5 * dist_sq).exp() / (2.0 * std::f64::consts::PI).sqrt()
                    };

                    density += k_val;
                }

                // Normalize by bandwidth and number of points
                density /= h * h * points.len() as f64;

                let idx = row * cols + col;
                output.data.set_f64(idx, density);
            }

            let progress = (row as f64 + 1.0) / rows as f64;
            ctx.progress.progress(progress);
        }

        ctx.progress.info("Writing raster output");
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RipleysKTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ripleys_k_test",
            display_name: "Ripley's K Function",
            summary: "Computes Ripley's K statistic for point pattern analysis.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "max_distance", description: "Maximum analysis distance.", required: false },
                ToolParamSpec { name: "step_size", description: "Distance step size for computation.", required: false },
                ToolParamSpec { name: "output", description: "Output vector with K statistics.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("k_results.gpkg"));

        ToolManifest {
            id: "ripleys_k_test".to_string(),
            display_name: "Ripley's K Function".to_string(),
            summary: "Computes Ripley's K statistic for point pattern clustering analysis.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "max_distance".to_string(), description: "Maximum analysis distance.".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Distance step size.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with K statistics.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "ripley_k_basic".to_string(),
                description: "Compute Ripley's K function.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "point-pattern".to_string(), "ripley-k".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let max_distance = parse_optional_f64_arg(args, "max_distance");
        let step_size = parse_optional_f64_arg(args, "step_size").unwrap_or(1.0);

        // Extract point coordinates
        let mut points = Vec::new();
        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    points.push((coord.x, coord.y));
                }
            }
        }

        if points.len() < 2 {
            return Err(ToolError::Execution(
                "Need at least 2 points for Ripley's K analysis".to_string(),
            ));
        }

        ctx.progress.info(&format!("Computing Ripley's K for {} points", points.len()));

        // Compute bounding box
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for (x, y) in &points {
            min_x = min_x.min(*x);
            max_x = max_x.max(*x);
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }

        let width = max_x - min_x;
        let height = max_y - min_y;
        let area = width * height;
        let lambda = points.len() as f64 / area; // Intensity

        // Determine max distance (use 1/4 of minimum dimension if not provided)
        let max_dist = max_distance.unwrap_or((width.min(height)) / 4.0);

        // Compute all pairwise distances
        let n = points.len();
        let mut distances = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = points[i].0 - points[j].0;
                let dy = points[i].1 - points[j].1;
                let dist = (dx * dx + dy * dy).sqrt();
                distances.push(dist);
            }
        }

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Create output layer with schema
        let mut output_layer = input.clone();
        let mut schema = wbvector::Schema::new();

        // Add K statistic fields for each distance
        let mut distance_vals = Vec::new();
        let mut r = step_size;
        while r <= max_dist {
            let field_name = format!("k_r_{:.2}", r);
            schema.add_field(wbvector::FieldDef::new(&field_name, wbvector::FieldType::Float));
            distance_vals.push(r);
            r += step_size;
        }

        output_layer.schema = schema;

        // Compute K(r) for each distance
        let mut k_values = Vec::new();
        for &r in &distance_vals {
            // Count pairs with distance <= r
            let count = distances.iter().filter(|&&d| d <= r).count();
            let k_r = (area / (lambda * n as f64 * n as f64)) * (2.0 * count as f64);
            k_values.push(k_r);
        }

        // Create a single output feature with K values
        let mut feature = wbvector::Feature::new();
        feature.geometry = Some(wbvector::Geometry::point(min_x, min_y)); // Dummy geometry

        for (i, &k_val) in k_values.iter().enumerate() {
            feature.attributes.insert(
                i,
                wbvector::FieldValue::Float(k_val as f64),
            );
        }

        output_layer.features.push(feature);

        let locator = write_vector_output(&output_layer, output_path.as_str())?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for EnvelopeTestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "envelope_test",
            display_name: "Envelope Test",
            summary: "Computes simulation envelope for point pattern significance testing.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "num_simulations", description: "Number of Monte Carlo simulations (default 99).", required: false },
                ToolParamSpec { name: "max_distance", description: "Maximum analysis distance.", required: false },
                ToolParamSpec { name: "output", description: "Output vector with envelope bounds.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("num_simulations".to_string(), json!(99));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("envelope_results.gpkg"));

        ToolManifest {
            id: "envelope_test".to_string(),
            display_name: "Envelope Test".to_string(),
            summary: "Computes simulation envelope for significance testing of point patterns.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "num_simulations".to_string(), description: "Number of Monte Carlo simulations.".to_string(), required: false },
                ToolParamDescriptor { name: "max_distance".to_string(), description: "Maximum analysis distance.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with envelope bounds.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "envelope_test_basic".to_string(),
                description: "Compute envelope test for point pattern analysis.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "point-pattern".to_string(), "envelope".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        use std::collections::BTreeMap;

        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let num_sims = parse_optional_usize_arg(args, "num_simulations")?.unwrap_or(99);
        let max_distance = parse_optional_f64_arg(args, "max_distance");
        let step_size = 1.0; // Fixed step size for output

        // Extract point coordinates
        let mut points = Vec::new();
        for feature in &input.features {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    points.push((coord.x, coord.y));
                }
            }
        }

        if points.len() < 2 {
            return Err(ToolError::Execution(
                "Need at least 2 points for envelope test".to_string(),
            ));
        }

        ctx.progress.info(&format!(
            "Computing envelope test with {} simulations for {} points",
            num_sims,
            points.len()
        ));

        // Compute bounding box
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for (x, y) in &points {
            min_x = min_x.min(*x);
            max_x = max_x.max(*x);
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }

        let width = max_x - min_x;
        let height = max_y - min_y;
        let area = width * height;
        let lambda = points.len() as f64 / area;
        let max_dist = max_distance.unwrap_or((width.min(height)) / 4.0);

        // Compute observed K
        let n = points.len();
        let mut obs_distances = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = points[i].0 - points[j].0;
                let dy = points[i].1 - points[j].1;
                let dist = (dx * dx + dy * dy).sqrt();
                obs_distances.push(dist);
            }
        }
        obs_distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Store simulation results keyed by distance
        let mut sim_k_values: BTreeMap<u32, Vec<f64>> = BTreeMap::new();

        // Run simulations
        for sim_idx in 0..num_sims {
            let progress = sim_idx as f64 / num_sims as f64;
            ctx.progress.progress(progress);

            // Generate random points in same area
            let mut sim_points = Vec::new();
            let mut rng = std::num::Wrapping(12345u64 + sim_idx as u64);
            for _ in 0..n {
                // Simple LCG random number generator
                rng = rng * std::num::Wrapping(1103515245) + std::num::Wrapping(12345);
                let u1 = ((rng.0 / 65536) % 32768) as f64 / 32768.0;
                rng = rng * std::num::Wrapping(1103515245) + std::num::Wrapping(12345);
                let u2 = ((rng.0 / 65536) % 32768) as f64 / 32768.0;

                let sim_x = min_x + u1 * width;
                let sim_y = min_y + u2 * height;
                sim_points.push((sim_x, sim_y));
            }

            // Compute K for simulated points
            let mut sim_distances = Vec::new();
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = sim_points[i].0 - sim_points[j].0;
                    let dy = sim_points[i].1 - sim_points[j].1;
                    let dist = (dx * dx + dy * dy).sqrt();
                    sim_distances.push(dist);
                }
            }
            sim_distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // Store K values at each distance
            let mut r = step_size;
            while r <= max_dist {
                let r_key = (r * 100.0) as u32; // Convert to fixed-point key
                let count = sim_distances.iter().filter(|&&d| d <= r).count();
                let k_r = (area / (lambda * n as f64 * n as f64)) * (2.0 * count as f64);

                sim_k_values.entry(r_key).or_insert_with(Vec::new).push(k_r);
                r += step_size;
            }
        }

        // Create output schema
        let mut output_layer = input.clone();
        let mut schema = wbvector::Schema::new();

        schema.add_field(wbvector::FieldDef::new("distance", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("k_observed", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("k_lower", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("k_upper", wbvector::FieldType::Float));

        output_layer.schema = schema;

        // Create output features with envelope bounds
        let mut r = step_size;
        while r <= max_dist {
            let r_key = (r * 100.0) as u32;

            // Observed K value
            let count_obs = obs_distances.iter().filter(|&&d| d <= r).count();
            let k_obs = (area / (lambda * n as f64 * n as f64)) * (2.0 * count_obs as f64);

            // Compute percentiles from simulations
            let k_lower = if let Some(sims) = sim_k_values.get(&r_key) {
                let mut sorted = sims.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = ((num_sims as f64) * 0.05) as usize;
                sorted.get(idx).copied().unwrap_or(k_obs)
            } else {
                k_obs
            };

            let k_upper = if let Some(sims) = sim_k_values.get(&r_key) {
                let mut sorted = sims.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = ((num_sims as f64) * 0.95) as usize;
                sorted.get(idx).copied().unwrap_or(k_obs)
            } else {
                k_obs
            };

            let mut feature = wbvector::Feature::new();
            feature.geometry = Some(wbvector::Geometry::point(min_x, min_y));

            feature.attributes.insert(0, wbvector::FieldValue::Float(r as f64));
            feature.attributes.insert(1, wbvector::FieldValue::Float(k_obs as f64));
            feature.attributes.insert(2, wbvector::FieldValue::Float(k_lower as f64));
            feature.attributes.insert(3, wbvector::FieldValue::Float(k_upper as f64));

            output_layer.features.push(feature);
            r += step_size;
        }

        let locator = write_vector_output(&output_layer, output_path.as_str())?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for PointProcessResidualsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "point_process_residuals",
            display_name: "Point Process Residuals",
            summary: "Computes residuals from fitted Poisson point process model.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "intensity_field", description: "Fitted intensity field (optional; computes residuals).", required: false },
                ToolParamSpec { name: "output", description: "Output vector with residual values.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("residuals.gpkg"));

        ToolManifest {
            id: "point_process_residuals".to_string(),
            display_name: "Point Process Residuals".to_string(),
            summary: "Computes residuals from fitted Poisson point process model.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "intensity_field".to_string(), description: "Fitted intensity field.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with residuals.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "residuals_basic".to_string(),
                description: "Compute point process residuals.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "point-pattern".to_string(), "residuals".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let intensity_field = parse_optional_string_arg(args, "intensity_field")?;

        // Extract point coordinates
        let mut points = Vec::new();
        let mut fitted_intensities = Vec::new();

        for (_idx, feature) in input.features.iter().enumerate() {
            if let Some(geom) = &feature.geometry {
                if let wbvector::Geometry::Point(coord) = geom {
                    points.push((coord.x, coord.y));

                    // Get fitted intensity if field provided
                    let fitted_int = if let Some(field_name) = &intensity_field {
                        if let Some(field_idx) = input.schema.field_index(field_name) {
                            if let Some(attr) = feature.attributes.get(field_idx) {
                                match attr {
                                    wbvector::FieldValue::Float(v) => *v as f64,
                                    wbvector::FieldValue::Integer(v) => *v as f64,
                                    _ => 0.0,
                                }
                            } else {
                                0.0
                            }
                        } else {
                            return Err(ToolError::Execution(format!(
                                "Intensity field '{}' not found",
                                field_name
                            )));
                        }
                    } else {
                        // Estimate as mean intensity (number of points / area)
                        1.0 / (points.len() as f64)
                    };

                    fitted_intensities.push(fitted_int);
                }
            }
        }

        if points.is_empty() {
            return Err(ToolError::Execution("No valid point features found".to_string()));
        }

        ctx.progress.info(&format!(
            "Computing point process residuals for {} points",
            points.len()
        ));

        // If intensity field not provided, compute KDE as intensity estimate
        if intensity_field.is_none() {
            // Compute bounding box
            let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
            for (x, y) in &points {
                min_x = min_x.min(*x);
                max_x = max_x.max(*x);
                min_y = min_y.min(*y);
                max_y = max_y.max(*y);
            }

            let width = max_x - min_x;
            let height = max_y - min_y;
            let _area = width * height;

            // Scott's rule bandwidth
            let n = points.len() as f64;
            let h = n.powf(-1.0 / 3.0) * 10.0;

            // Compute KDE intensity at each point location
            for (i, &(x, y)) in points.iter().enumerate() {
                let mut density = 0.0;
                for (px, py) in &points {
                    let dx = (x - px) / h;
                    let dy = (y - py) / h;
                    let dist_sq = dx * dx + dy * dy;
                    let k_val = (-0.5 * dist_sq).exp() / (2.0 * std::f64::consts::PI).sqrt();
                    density += k_val;
                }
                density /= h * h * points.len() as f64;
                fitted_intensities[i] = density;
            }
        }

        // Create output layer
        let mut output_layer = input.clone();
        let mut schema = output_layer.schema.clone();

        schema.add_field(wbvector::FieldDef::new("fitted_intensity", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("raw_residual", wbvector::FieldType::Float));
        schema.add_field(wbvector::FieldDef::new("std_residual", wbvector::FieldType::Float));

        output_layer.schema = schema;

        // Compute residuals
        let fitted_intensity_idx = output_layer.schema.field_index("fitted_intensity").unwrap();
        let raw_residual_idx = output_layer.schema.field_index("raw_residual").unwrap();
        let std_residual_idx = output_layer.schema.field_index("std_residual").unwrap();

        for (idx, feature) in input.features.iter().enumerate() {
            let mut new_feature = feature.clone();

            let fitted = fitted_intensities[idx];
            let raw_residual = 1.0 - fitted; // Observed (1 for point present) - fitted
            let std_residual = if fitted > 0.0 {
                raw_residual / fitted.sqrt()
            } else {
                raw_residual
            };

            new_feature.attributes.insert(
                fitted_intensity_idx,
                wbvector::FieldValue::Float(fitted as f64),
            );
            new_feature.attributes.insert(
                raw_residual_idx,
                wbvector::FieldValue::Float(raw_residual as f64),
            );
            new_feature.attributes.insert(
                std_residual_idx,
                wbvector::FieldValue::Float(std_residual as f64),
            );

            output_layer.features.push(new_feature);
        }

        let locator = write_vector_output(&output_layer, output_path.as_str())?;

        let mut outputs = ToolArgs::new();
        outputs.insert("output".to_string(), json!(locator));

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs })
    }
}
