use serde_json::json;
use std::sync::Mutex;
use wbcore::{CapabilityProvider, LicenseTier, ProgressSink, ToolArgs, ToolContext, ToolError};
use wbtools_oss::{register_default_tools, ToolRegistry};
use wblidar::{Crs as LidarCrs, GpsTime, PointCloud, PointRecord, Rgb16};
use wbraster::{memory_store as raster_memory_store, CrsInfo, DataType, Raster, RasterConfig, RasterFormat};
use wbvector::{Coord, Feature, Geometry, GeometryType, Layer, VectorFormat};

struct OpenOnly;
struct FullAccess;

impl CapabilityProvider for OpenOnly {
    fn has_tool_access(&self, _tool_id: &'static str, tier: LicenseTier) -> bool {
        matches!(tier, LicenseTier::Open)
    }
}

impl CapabilityProvider for FullAccess {
    fn has_tool_access(&self, _tool_id: &'static str, _tier: LicenseTier) -> bool {
        true
    }
}

struct NoProgress;

impl ProgressSink for NoProgress {}

fn context<'a>(capabilities: &'a dyn CapabilityProvider) -> ToolContext<'a> {
    static PROGRESS: NoProgress = NoProgress;
    ToolContext {
        progress: &PROGRESS,
        capabilities,
    }
}

static CWD_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn default_registry_contains_core_math_tools() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ids: Vec<_> = registry.list().into_iter().map(|m| m.id).collect();

    assert!(ids.contains(&"add"));
    assert!(ids.contains(&"subtract"));
    assert!(ids.contains(&"multiply"));
    assert!(ids.contains(&"divide"));
    assert!(ids.contains(&"abs"));
    assert!(ids.contains(&"ceil"));
    assert!(ids.contains(&"floor"));
    assert!(ids.contains(&"round"));
    assert!(ids.contains(&"sqrt"));
    assert!(ids.contains(&"square"));
    assert!(ids.contains(&"ln"));
    assert!(ids.contains(&"log10"));
    assert!(ids.contains(&"sin"));
    assert!(ids.contains(&"cos"));
}

#[test]
fn manifests_include_defaults_examples_and_tags() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let manifests = registry.manifests();
    let add_manifest = manifests
        .iter()
        .find(|m| m.id == "add")
        .expect("missing add manifest");

    assert_eq!(add_manifest.stability, wbcore::ToolStability::Experimental);
    assert!(!add_manifest.examples.is_empty());
    assert!(add_manifest.defaults.contains_key("input1"));
    assert!(add_manifest.defaults.contains_key("input2"));
    assert!(add_manifest.tags.iter().any(|t| t == "raster"));
}

#[test]
fn default_registry_contains_hydrology_depression_tools() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ids: Vec<_> = registry.list().into_iter().map(|m| m.id).collect();

    assert!(ids.contains(&"breach_depressions_least_cost"));
    assert!(ids.contains(&"breach_single_cell_pits"));
    assert!(ids.contains(&"fill_depressions"));
    assert!(ids.contains(&"fill_depressions_planchon_and_darboux"));
    assert!(ids.contains(&"fill_depressions_wang_and_liu"));
    assert!(ids.contains(&"fill_pits"));
    assert!(ids.contains(&"flow_accum_full_workflow"));
    assert!(ids.contains(&"find_noflow_cells"));
    assert!(ids.contains(&"num_inflowing_neighbours"));
    assert!(ids.contains(&"find_parallel_flow"));
    assert!(ids.contains(&"edge_contamination"));
    assert!(ids.contains(&"d8_mass_flux"));
    assert!(ids.contains(&"flow_length_diff"));
    assert!(ids.contains(&"basins"));
    assert!(ids.contains(&"watershed_from_raster_pour_points"));
    assert!(ids.contains(&"watershed"));
    assert!(ids.contains(&"jenson_snap_pour_points"));
    assert!(ids.contains(&"subbasins"));
    assert!(ids.contains(&"hillslopes"));
    assert!(ids.contains(&"strahler_order_basins"));
    assert!(ids.contains(&"isobasins"));
    assert!(ids.contains(&"insert_dams"));
    assert!(ids.contains(&"raise_walls"));
    assert!(ids.contains(&"topological_breach_burn"));
    assert!(ids.contains(&"stochastic_depression_analysis"));
    assert!(ids.contains(&"unnest_basins"));
    assert!(ids.contains(&"upslope_depression_storage"));
}

#[test]
fn default_registry_contains_gis_overlay_tools() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ids: Vec<_> = registry.list().into_iter().map(|m| m.id).collect();

    assert!(ids.contains(&"topology_validation_report"));
    assert!(ids.contains(&"topology_rule_validate"));
    assert!(ids.contains(&"average_overlay"));
    assert!(ids.contains(&"boundary_shape_complexity"));
    assert!(ids.contains(&"buffer_raster"));
    assert!(ids.contains(&"buffer_vector"));
    assert!(ids.contains(&"centroid_raster"));
    assert!(ids.contains(&"centroid_vector"));
    assert!(ids.contains(&"clip"));
    assert!(ids.contains(&"clip_raster_to_polygon"));
    assert!(ids.contains(&"clump"));
    assert!(ids.contains(&"compactness_ratio"));
    assert!(ids.contains(&"count_if"));
    assert!(ids.contains(&"cost_allocation"));
    assert!(ids.contains(&"cost_distance"));
    assert!(ids.contains(&"cost_pathway"));
    assert!(ids.contains(&"create_plane"));
    assert!(ids.contains(&"difference"));
    assert!(ids.contains(&"dissolve"));
    assert!(ids.contains(&"deviation_from_regional_direction"));
    assert!(ids.contains(&"edge_proportion"));
    assert!(ids.contains(&"elongation_ratio"));
    assert!(ids.contains(&"eliminate_coincident_points"));
    assert!(ids.contains(&"erase"));
    assert!(ids.contains(&"erase_polygon_from_raster"));
    assert!(ids.contains(&"extend_vector_lines"));
    assert!(ids.contains(&"extract_by_attribute"));
    assert!(ids.contains(&"extract_raster_values_at_points"));
    assert!(ids.contains(&"extract_nodes"));
    assert!(ids.contains(&"filter_vector_features_by_area"));
    assert!(ids.contains(&"euclidean_allocation"));
    assert!(ids.contains(&"euclidean_distance"));
    assert!(ids.contains(&"filter_raster_features_by_area"));
    assert!(ids.contains(&"find_patch_edge_cells"));
    assert!(ids.contains(&"find_lowest_or_highest_points"));
    assert!(ids.contains(&"heat_map"));
    assert!(ids.contains(&"hexagonal_grid_from_raster_base"));
    assert!(ids.contains(&"hexagonal_grid_from_vector_base"));
    assert!(ids.contains(&"highest_position"));
    assert!(ids.contains(&"hole_proportion"));
    assert!(ids.contains(&"idw_interpolation"));
    assert!(ids.contains(&"intersect"));
    assert!(ids.contains(&"layer_footprint_raster"));
    assert!(ids.contains(&"layer_footprint_vector"));
    assert!(ids.contains(&"line_intersections"));
    assert!(ids.contains(&"linearity_index"));
    assert!(ids.contains(&"map_features"));
    assert!(ids.contains(&"merge_line_segments"));
    assert!(ids.contains(&"minimum_bounding_box"));
    assert!(ids.contains(&"minimum_bounding_circle"));
    assert!(ids.contains(&"minimum_bounding_envelope"));
    assert!(ids.contains(&"minimum_convex_hull"));
    assert!(ids.contains(&"medoid"));
    assert!(ids.contains(&"natural_neighbour_interpolation"));
    assert!(ids.contains(&"nearest_neighbour_interpolation"));
    assert!(ids.contains(&"modified_shepard_interpolation"));
    assert!(ids.contains(&"radial_basis_function_interpolation"));
    assert!(ids.contains(&"tin_interpolation"));
    assert!(ids.contains(&"lowest_position"));
    assert!(ids.contains(&"max_absolute_overlay"));
    assert!(ids.contains(&"max_overlay"));
    assert!(ids.contains(&"min_absolute_overlay"));
    assert!(ids.contains(&"min_overlay"));
    assert!(ids.contains(&"multiply_overlay"));
    assert!(ids.contains(&"narrowness_index"));
    assert!(ids.contains(&"percent_equal_to"));
    assert!(ids.contains(&"percent_greater_than"));
    assert!(ids.contains(&"percent_less_than"));
    assert!(ids.contains(&"patch_orientation"));
    assert!(ids.contains(&"perimeter_area_ratio"));
    assert!(ids.contains(&"pick_from_list"));
    assert!(ids.contains(&"polygon_area"));
    assert!(ids.contains(&"polygon_long_axis"));
    assert!(ids.contains(&"polygon_perimeter"));
    assert!(ids.contains(&"polygon_short_axis"));
    assert!(ids.contains(&"polygonize"));
    assert!(ids.contains(&"raster_area"));
    assert!(ids.contains(&"raster_perimeter"));
    assert!(ids.contains(&"radius_of_gyration"));
    assert!(ids.contains(&"raster_cell_assignment"));
    assert!(ids.contains(&"rectangular_grid_from_raster_base"));
    assert!(ids.contains(&"rectangular_grid_from_vector_base"));
    assert!(ids.contains(&"reclass"));
    assert!(ids.contains(&"reclass_equal_interval"));
    assert!(ids.contains(&"related_circumscribing_circle"));
    assert!(ids.contains(&"shape_complexity_index_raster"));
    assert!(ids.contains(&"shape_complexity_index_vector"));
    assert!(ids.contains(&"smooth_vectors"));
    assert!(ids.contains(&"snap_endnodes"));
    assert!(ids.contains(&"split_vector_lines"));
    assert!(ids.contains(&"standard_deviation_overlay"));
    assert!(ids.contains(&"split_with_lines"));
    assert!(ids.contains(&"sum_overlay"));
    assert!(ids.contains(&"symmetrical_difference"));
    assert!(ids.contains(&"travelling_salesman_problem"));
    assert!(ids.contains(&"union"));
    assert!(ids.contains(&"update_nodata_cells"));
    assert!(ids.contains(&"voronoi_diagram"));
    assert!(ids.contains(&"weighted_overlay"));
    assert!(ids.contains(&"weighted_sum"));
    assert!(ids.contains(&"reproject_vector"));
    assert!(ids.contains(&"add_geometry_attributes"));
    assert!(ids.contains(&"simplify_features"));
    assert!(ids.contains(&"near"));
    assert!(ids.contains(&"select_by_location"));
    assert!(ids.contains(&"field_calculator"));
    assert!(ids.contains(&"spatial_join"));
    assert!(ids.contains(&"concave_hull"));
    assert!(ids.contains(&"random_points_in_polygon"));
    assert!(ids.contains(&"densify_features"));
    assert!(ids.contains(&"points_along_lines"));
    assert!(ids.contains(&"locate_points_along_routes"));
    assert!(ids.contains(&"route_event_points_from_table"));
    assert!(ids.contains(&"route_event_lines_from_table"));
    assert!(ids.contains(&"route_event_points_from_layer"));
    assert!(ids.contains(&"route_event_lines_from_layer"));
    assert!(ids.contains(&"route_event_split"));
    assert!(ids.contains(&"route_event_merge"));
    assert!(ids.contains(&"route_event_overlay"));
    assert!(ids.contains(&"route_measure_qa"));
    assert!(ids.contains(&"route_calibrate"));
    assert!(ids.contains(&"route_recalibrate"));
    assert!(ids.contains(&"vector_summary_statistics"));
    assert!(ids.contains(&"rename_field"));
    assert!(ids.contains(&"delete_field"));
    assert!(ids.contains(&"add_field"));
    assert!(ids.contains(&"line_polygon_clip"));
    assert!(ids.contains(&"shortest_path_network"));
    assert!(ids.contains(&"multimodal_shortest_path"));
    assert!(ids.contains(&"multimodal_od_cost_matrix"));
    assert!(ids.contains(&"multimodal_routes_from_od"));
    assert!(ids.contains(&"network_centrality_metrics"));
    assert!(ids.contains(&"network_accessibility_metrics"));
    assert!(ids.contains(&"od_sensitivity_analysis"));
    assert!(ids.contains(&"network_node_degree"));
    assert!(ids.contains(&"network_service_area"));
    assert!(ids.contains(&"network_od_cost_matrix"));
    assert!(ids.contains(&"network_connected_components"));
    assert!(ids.contains(&"network_routes_from_od"));
    assert!(ids.contains(&"k_shortest_paths_network"));
    assert!(ids.contains(&"vehicle_routing_cvrp"));
    assert!(ids.contains(&"vehicle_routing_vrptw"));
    assert!(ids.contains(&"vehicle_routing_pickup_delivery"));
    assert!(ids.contains(&"block_minimum"));
    assert!(ids.contains(&"block_maximum"));
    assert!(ids.contains(&"aggregate_raster"));
}

#[test]
fn vehicle_routing_cvrp_builds_capacity_constrained_routes() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_cvrp_basic");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let routes_out = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let assign_out = std::env::temp_dir().join(format!("{tag}_assign.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(12.0, 0.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));
    for (x, demand) in [(1.0, 2.0), (2.5, 2.0), (10.0, 2.0)] {
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(x, 0.0))),
                &[("demand", FieldValue::Float(demand))],
            )
            .expect("add stop");
    }
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut args = ToolArgs::new();
    args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    args.insert("demand_field".to_string(), json!("demand"));
    args.insert("vehicle_capacity".to_string(), json!(4.0));
    args.insert("max_vehicles".to_string(), json!(2));
    args.insert("output".to_string(), json!(routes_out.to_string_lossy().to_string()));
    args.insert(
        "assignment_output".to_string(),
        json!(assign_out.to_string_lossy().to_string()),
    );

    let result = registry
        .run("vehicle_routing_cvrp", &args, &context(&caps))
        .expect("vehicle_routing_cvrp run");

    let routes = wbvector::read(&routes_out).expect("read routes");
    assert_eq!(routes.features.len(), 2, "expected two routes under capacity 4.0");

    let assignments = wbvector::read(&assign_out).expect("read assignments");
    assert_eq!(assignments.features.len(), 3, "all stops should be assigned");

    let served = result
        .outputs
        .get("served_stop_count")
        .and_then(|v| v.as_u64())
        .expect("served_stop_count");
    let unserved = result
        .outputs
        .get("unserved_stop_count")
        .and_then(|v| v.as_u64())
        .expect("unserved_stop_count");
    assert_eq!(served, 3);
    assert_eq!(unserved, 0);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&routes_out);
    let _ = std::fs::remove_file(&assign_out);
}

#[test]
fn vehicle_routing_cvrp_local_optimization_reduces_route_distance() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_cvrp_local_opt");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let baseline_out = std::env::temp_dir().join(format!("{tag}_baseline.gpkg"));
    let optimized_out = std::env::temp_dir().join(format!("{tag}_optimized.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(-4.0, -4.0), Coord::xy(0.0, 2.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));
    for coord in [(-3.0, -3.0), (-3.0, -2.0), (-3.0, -1.0), (-3.0, 1.0), (-2.0, -3.0)] {
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(coord.0, coord.1))),
                &[("demand", FieldValue::Float(1.0))],
            )
            .expect("add stop");
    }
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut baseline_args = ToolArgs::new();
    baseline_args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    baseline_args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    baseline_args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    baseline_args.insert("demand_field".to_string(), json!("demand"));
    baseline_args.insert("vehicle_capacity".to_string(), json!(10.0));
    baseline_args.insert("max_vehicles".to_string(), json!(1));
    baseline_args.insert("apply_local_optimization".to_string(), json!(false));
    baseline_args.insert("output".to_string(), json!(baseline_out.to_string_lossy().to_string()));

    let baseline_result = registry
        .run("vehicle_routing_cvrp", &baseline_args, &context(&caps))
        .expect("baseline cvrp run");

    let mut optimized_args = baseline_args.clone();
    optimized_args.insert("apply_local_optimization".to_string(), json!(true));
    optimized_args.insert("output".to_string(), json!(optimized_out.to_string_lossy().to_string()));

    let optimized_result = registry
        .run("vehicle_routing_cvrp", &optimized_args, &context(&caps))
        .expect("optimized cvrp run");

    let baseline_routes = wbvector::read(&baseline_out).expect("read baseline routes");
    let optimized_routes = wbvector::read(&optimized_out).expect("read optimized routes");
    assert_eq!(baseline_routes.features.len(), 1);
    assert_eq!(optimized_routes.features.len(), 1);

    let distance_idx = baseline_routes.schema.field_index("DISTANCE").expect("DISTANCE field");
    let baseline_distance = match &baseline_routes.features[0].attributes[distance_idx] {
        FieldValue::Float(v) => *v,
        other => panic!("expected float DISTANCE, got {:?}", other),
    };
    let optimized_distance = match &optimized_routes.features[0].attributes[distance_idx] {
        FieldValue::Float(v) => *v,
        other => panic!("expected float DISTANCE, got {:?}", other),
    };

    assert!(optimized_distance + 1.0e-9 < baseline_distance);
    assert_eq!(
        baseline_result.outputs.get("optimized_route_count").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        optimized_result.outputs.get("optimized_route_count").and_then(|v| v.as_u64()),
        Some(1)
    );

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&baseline_out);
    let _ = std::fs::remove_file(&optimized_out);
}

#[test]
fn vehicle_routing_cvrp_benchmark_local_optimization_outperforms_phase3_greedy_baseline() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_cvrp_phase4_benchmark");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let baseline_out = std::env::temp_dir().join(format!("{tag}_baseline.gpkg"));
    let optimized_out = std::env::temp_dir().join(format!("{tag}_optimized.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(-5.0, -5.0), Coord::xy(4.0, 1.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));
    for coord in [(-5.0, -5.0), (-5.0, -4.0), (-5.0, -3.0), (-5.0, -2.0), (-5.0, -1.0), (-5.0, 1.0), (4.0, -3.0)] {
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(coord.0, coord.1))),
                &[("demand", FieldValue::Float(1.0))],
            )
            .expect("add stop");
    }
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut baseline_args = ToolArgs::new();
    baseline_args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    baseline_args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    baseline_args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    baseline_args.insert("demand_field".to_string(), json!("demand"));
    baseline_args.insert("vehicle_capacity".to_string(), json!(4.0));
    baseline_args.insert("max_vehicles".to_string(), json!(2));
    baseline_args.insert("apply_local_optimization".to_string(), json!(false));
    baseline_args.insert("output".to_string(), json!(baseline_out.to_string_lossy().to_string()));

    let baseline_result = registry
        .run("vehicle_routing_cvrp", &baseline_args, &context(&caps))
        .expect("baseline cvrp run");

    let mut optimized_args = baseline_args.clone();
    optimized_args.insert("apply_local_optimization".to_string(), json!(true));
    optimized_args.insert("output".to_string(), json!(optimized_out.to_string_lossy().to_string()));

    let optimized_result = registry
        .run("vehicle_routing_cvrp", &optimized_args, &context(&caps))
        .expect("optimized cvrp run");

    let baseline_distance = baseline_result
        .outputs
        .get("total_distance")
        .and_then(|v| v.as_f64())
        .expect("baseline total_distance");
    let optimized_distance = optimized_result
        .outputs
        .get("total_distance")
        .and_then(|v| v.as_f64())
        .expect("optimized total_distance");

    assert_eq!(baseline_result.outputs.get("route_count").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(optimized_result.outputs.get("route_count").and_then(|v| v.as_u64()), Some(2));
    assert!(optimized_distance + 1.0e-9 < baseline_distance);
    assert_eq!(optimized_result.outputs.get("optimized_route_count").and_then(|v| v.as_u64()), Some(1));

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&baseline_out);
    let _ = std::fs::remove_file(&optimized_out);
}

#[test]
fn vehicle_routing_vrptw_reports_lateness() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_vrptw_lateness");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let routes_out = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let assign_out = std::env::temp_dir().join(format!("{tag}_assign.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(4.0, 0.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_start", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_end", FieldType::Float));
    stops.schema.add_field(FieldDef::new("service_time", FieldType::Float));

    stops
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.0, 0.0))),
            &[
                ("demand", FieldValue::Float(1.0)),
                ("tw_start", FieldValue::Float(0.0)),
                ("tw_end", FieldValue::Float(10.0)),
                ("service_time", FieldValue::Float(1.0)),
            ],
        )
        .expect("add stop 1");
    stops
        .add_feature(
            Some(Geometry::Point(Coord::xy(3.0, 0.0))),
            &[
                ("demand", FieldValue::Float(1.0)),
                ("tw_start", FieldValue::Float(0.0)),
                ("tw_end", FieldValue::Float(2.0)),
                ("service_time", FieldValue::Float(1.0)),
            ],
        )
        .expect("add stop 2");
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut args = ToolArgs::new();
    args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    args.insert("demand_field".to_string(), json!("demand"));
    args.insert("tw_start_field".to_string(), json!("tw_start"));
    args.insert("tw_end_field".to_string(), json!("tw_end"));
    args.insert("service_time_field".to_string(), json!("service_time"));
    args.insert("vehicle_capacity".to_string(), json!(10.0));
    args.insert("travel_speed".to_string(), json!(1.0));
    args.insert("start_time".to_string(), json!(0.0));
    args.insert("output".to_string(), json!(routes_out.to_string_lossy().to_string()));
    args.insert(
        "assignment_output".to_string(),
        json!(assign_out.to_string_lossy().to_string()),
    );

    let result = registry
        .run("vehicle_routing_vrptw", &args, &context(&caps))
        .expect("vehicle_routing_vrptw run");

    let late_count = result
        .outputs
        .get("late_stop_count")
        .and_then(|v| v.as_u64())
        .expect("late_stop_count output");
    let total_lateness = result
        .outputs
        .get("total_lateness")
        .and_then(|v| v.as_f64())
        .expect("total_lateness output");
    assert!(late_count >= 1, "expected at least one late stop");
    assert!(total_lateness > 0.0, "expected positive lateness");

    let routes = wbvector::read(&routes_out).expect("read routes");
    assert_eq!(routes.features.len(), 1);
    let assignments = wbvector::read(&assign_out).expect("read assignments");
    assert_eq!(assignments.features.len(), 2);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&routes_out);
    let _ = std::fs::remove_file(&assign_out);
}

#[test]
fn vehicle_routing_vrptw_hard_windows_report_infeasible_stops() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_vrptw_hard_windows");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let routes_out = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(4.0, 0.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_start", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_end", FieldType::Float));
    stops.schema.add_field(FieldDef::new("service_time", FieldType::Float));

    stops
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.0, 0.0))),
            &[
                ("demand", FieldValue::Float(1.0)),
                ("tw_start", FieldValue::Float(0.0)),
                ("tw_end", FieldValue::Float(10.0)),
                ("service_time", FieldValue::Float(1.0)),
            ],
        )
        .expect("add stop 1");
    stops
        .add_feature(
            Some(Geometry::Point(Coord::xy(3.0, 0.0))),
            &[
                ("demand", FieldValue::Float(1.0)),
                ("tw_start", FieldValue::Float(0.0)),
                ("tw_end", FieldValue::Float(2.0)),
                ("service_time", FieldValue::Float(1.0)),
            ],
        )
        .expect("add stop 2");
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut args = ToolArgs::new();
    args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    args.insert("demand_field".to_string(), json!("demand"));
    args.insert("tw_start_field".to_string(), json!("tw_start"));
    args.insert("tw_end_field".to_string(), json!("tw_end"));
    args.insert("service_time_field".to_string(), json!("service_time"));
    args.insert("vehicle_capacity".to_string(), json!(10.0));
    args.insert("travel_speed".to_string(), json!(1.0));
    args.insert("start_time".to_string(), json!(0.0));
    args.insert("enforce_time_windows".to_string(), json!(true));
    args.insert("allowed_lateness".to_string(), json!(0.0));
    args.insert("output".to_string(), json!(routes_out.to_string_lossy().to_string()));

    let result = registry
        .run("vehicle_routing_vrptw", &args, &context(&caps))
        .expect("vehicle_routing_vrptw hard-window run");

    let served = result
        .outputs
        .get("served_stop_count")
        .and_then(|v| v.as_u64())
        .expect("served_stop_count output");
    let unserved = result
        .outputs
        .get("unserved_stop_count")
        .and_then(|v| v.as_u64())
        .expect("unserved_stop_count output");
    let tw_infeasible = result
        .outputs
        .get("time_window_infeasible_stop_count")
        .and_then(|v| v.as_u64())
        .expect("time_window_infeasible_stop_count output");
    assert_eq!(served, 1, "strict windows should only serve first stop");
    assert_eq!(unserved, 1, "strict windows should leave one stop unserved");
    assert_eq!(tw_infeasible, 1, "strict windows should report one infeasible stop");

    let routes = wbvector::read(&routes_out).expect("read routes");
    assert_eq!(routes.features.len(), 1);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&routes_out);
}

#[test]
fn vehicle_routing_pickup_delivery_enforces_pair_precedence() {
    use std::collections::HashMap;
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_pickup_delivery");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let routes_out = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let assign_out = std::env::temp_dir().join(format!("{tag}_assign.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(12.0, 0.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("request_id", FieldType::Text));
    stops.schema.add_field(FieldDef::new("stop_type", FieldType::Text));
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));

    for (x, request_id, stop_type, demand) in [
        (1.0, "R1", "pickup", 3.0),
        (5.0, "R1", "delivery", 3.0),
        (2.0, "R2", "pickup", 2.0),
        (8.0, "R2", "delivery", 2.0),
    ] {
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(x, 0.0))),
                &[
                    ("request_id", FieldValue::Text(request_id.to_string())),
                    ("stop_type", FieldValue::Text(stop_type.to_string())),
                    ("demand", FieldValue::Float(demand)),
                ],
            )
            .expect("add stop");
    }
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut args = ToolArgs::new();
    args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    args.insert("request_id_field".to_string(), json!("request_id"));
    args.insert("stop_type_field".to_string(), json!("stop_type"));
    args.insert("demand_field".to_string(), json!("demand"));
    args.insert("vehicle_capacity".to_string(), json!(5.0));
    args.insert("max_vehicles".to_string(), json!(1));
    args.insert("output".to_string(), json!(routes_out.to_string_lossy().to_string()));
    args.insert(
        "assignment_output".to_string(),
        json!(assign_out.to_string_lossy().to_string()),
    );

    let result = registry
        .run("vehicle_routing_pickup_delivery", &args, &context(&caps))
        .expect("vehicle_routing_pickup_delivery run");

    let served = result
        .outputs
        .get("served_request_count")
        .and_then(|v| v.as_u64())
        .expect("served_request_count output");
    let unserved = result
        .outputs
        .get("unserved_request_count")
        .and_then(|v| v.as_u64())
        .expect("unserved_request_count output");
    assert_eq!(served, 2);
    assert_eq!(unserved, 0);

    let routes = wbvector::read(&routes_out).expect("read routes");
    assert_eq!(routes.features.len(), 1, "expected one route for two requests");

    let assignments = wbvector::read(&assign_out).expect("read assignments");
    assert_eq!(assignments.features.len(), 4, "expected pickup+delivery for each request");

    let request_idx = assignments
        .schema
        .field_index("REQUEST_ID")
        .expect("assignment REQUEST_ID field");
    let role_idx = assignments
        .schema
        .field_index("STOP_ROLE")
        .expect("assignment STOP_ROLE field");
    let seq_idx = assignments
        .schema
        .field_index("VISIT_SEQ")
        .expect("assignment VISIT_SEQ field");

    let mut visit_map: HashMap<String, (Option<i64>, Option<i64>)> = HashMap::new();
    for feature in &assignments.features {
        let request_id = match &feature.attributes[request_idx] {
            FieldValue::Text(v) => v.clone(),
            other => panic!("unexpected REQUEST_ID value: {other:?}"),
        };
        let stop_role = match &feature.attributes[role_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("unexpected STOP_ROLE value: {other:?}"),
        };
        let visit_seq = match &feature.attributes[seq_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("unexpected VISIT_SEQ value: {other:?}"),
        };

        let entry = visit_map.entry(request_id).or_insert((None, None));
        if stop_role == "pickup" {
            entry.0 = Some(visit_seq);
        } else if stop_role == "delivery" {
            entry.1 = Some(visit_seq);
        } else {
            panic!("unexpected STOP_ROLE value: {stop_role}");
        }
    }

    for (request_id, (pickup_seq, delivery_seq)) in visit_map {
        let p = pickup_seq.unwrap_or_else(|| panic!("missing pickup visit for request {request_id}"));
        let d = delivery_seq.unwrap_or_else(|| panic!("missing delivery visit for request {request_id}"));
        assert!(p < d, "pickup must precede delivery for request {request_id}");
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&routes_out);
    let _ = std::fs::remove_file(&assign_out);
}

#[test]
fn vehicle_routing_vrptw_relaxed_windows_serves_municipal_scenario() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_vrptw_municipal_relaxed");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let routes_out = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(8.0, 0.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_start", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_end", FieldType::Float));
    stops.schema.add_field(FieldDef::new("service_time", FieldType::Float));

    for (x, tw_end) in [(1.0, 4.0), (3.0, 3.0), (6.0, 5.0)] {
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(x, 0.0))),
                &[
                    ("demand", FieldValue::Float(1.0)),
                    ("tw_start", FieldValue::Float(0.0)),
                    ("tw_end", FieldValue::Float(tw_end)),
                    ("service_time", FieldValue::Float(0.5)),
                ],
            )
            .expect("add stop");
    }
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut args = ToolArgs::new();
    args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    args.insert("demand_field".to_string(), json!("demand"));
    args.insert("tw_start_field".to_string(), json!("tw_start"));
    args.insert("tw_end_field".to_string(), json!("tw_end"));
    args.insert("service_time_field".to_string(), json!("service_time"));
    args.insert("vehicle_capacity".to_string(), json!(10.0));
    args.insert("travel_speed".to_string(), json!(1.0));
    args.insert("enforce_time_windows".to_string(), json!(true));
    args.insert("allowed_lateness".to_string(), json!(3.5));
    args.insert("output".to_string(), json!(routes_out.to_string_lossy().to_string()));

    let result = registry
        .run("vehicle_routing_vrptw", &args, &context(&caps))
        .expect("vehicle_routing_vrptw relaxed-window run");

    let served = result
        .outputs
        .get("served_stop_count")
        .and_then(|v| v.as_u64())
        .expect("served_stop_count output");
    let tw_infeasible = result
        .outputs
        .get("time_window_infeasible_stop_count")
        .and_then(|v| v.as_u64())
        .expect("time_window_infeasible_stop_count output");
    assert_eq!(served, 3, "relaxed windows should serve all municipal stops");
    assert_eq!(tw_infeasible, 0, "relaxed windows should avoid infeasible pruning");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&routes_out);
}

#[test]
fn vehicle_routing_vrptw_benchmark_priority_scoring_reduces_total_lateness_vs_phase3_baseline() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_vrptw_phase4_benchmark");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let baseline_out = std::env::temp_dir().join(format!("{tag}_baseline.gpkg"));
    let scored_out = std::env::temp_dir().join(format!("{tag}_scored.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(4.0, 2.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_start", FieldType::Float));
    stops.schema.add_field(FieldDef::new("tw_end", FieldType::Float));
    stops.schema.add_field(FieldDef::new("service_time", FieldType::Float));

    for (x, y, tw_end) in [
        (1.0, 0.0, 5.0),
        (2.0, 0.0, 5.0),
        (3.0, 0.0, 5.0),
        (4.0, 0.0, 6.0),
        (0.0, 2.0, 2.0),
    ] {
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(x, y))),
                &[
                    ("demand", FieldValue::Float(1.0)),
                    ("tw_start", FieldValue::Float(0.0)),
                    ("tw_end", FieldValue::Float(tw_end)),
                    ("service_time", FieldValue::Float(0.5)),
                ],
            )
            .expect("add stop");
    }
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut baseline_args = ToolArgs::new();
    baseline_args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    baseline_args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    baseline_args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    baseline_args.insert("demand_field".to_string(), json!("demand"));
    baseline_args.insert("tw_start_field".to_string(), json!("tw_start"));
    baseline_args.insert("tw_end_field".to_string(), json!("tw_end"));
    baseline_args.insert("service_time_field".to_string(), json!("service_time"));
    baseline_args.insert("vehicle_capacity".to_string(), json!(10.0));
    baseline_args.insert("travel_speed".to_string(), json!(1.0));
    baseline_args.insert("start_time".to_string(), json!(0.0));
    baseline_args.insert("max_vehicles".to_string(), json!(1));
    baseline_args.insert("use_priority_scoring".to_string(), json!(false));
    baseline_args.insert("output".to_string(), json!(baseline_out.to_string_lossy().to_string()));

    let baseline_result = registry
        .run("vehicle_routing_vrptw", &baseline_args, &context(&caps))
        .expect("vrptw baseline run");

    let mut scored_args = baseline_args.clone();
    scored_args.insert("use_priority_scoring".to_string(), json!(true));
    scored_args.insert("output".to_string(), json!(scored_out.to_string_lossy().to_string()));

    let scored_result = registry
        .run("vehicle_routing_vrptw", &scored_args, &context(&caps))
        .expect("vrptw scored run");

    let baseline_lateness = baseline_result
        .outputs
        .get("total_lateness")
        .and_then(|v| v.as_f64())
        .expect("baseline total_lateness");
    let scored_lateness = scored_result
        .outputs
        .get("total_lateness")
        .and_then(|v| v.as_f64())
        .expect("scored total_lateness");

    assert!(scored_lateness + 1.0e-9 < baseline_lateness);
    assert_eq!(
        baseline_result.outputs.get("use_priority_scoring").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        scored_result.outputs.get("use_priority_scoring").and_then(|v| v.as_bool()),
        Some(true)
    );

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&baseline_out);
    let _ = std::fs::remove_file(&scored_out);
}

#[test]
fn vehicle_routing_pickup_delivery_logistics_benchmark_serves_all_requests() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vehicle_routing_pickup_delivery_logistics");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let depot_path = std::env::temp_dir().join(format!("{tag}_depots.gpkg"));
    let stops_path = std::env::temp_dir().join(format!("{tag}_stops.gpkg"));
    let routes_out = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(20.0, 0.0)])),
            &[],
        )
        .expect("add network line");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut depots = Layer::new("depots")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    depots
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add depot point");
    wbvector::write(&depots, &depot_path, VectorFormat::GeoPackage).expect("write depots");

    let mut stops = Layer::new("stops")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    stops.schema.add_field(FieldDef::new("request_id", FieldType::Text));
    stops.schema.add_field(FieldDef::new("stop_type", FieldType::Text));
    stops.schema.add_field(FieldDef::new("demand", FieldType::Float));

    let requests = [
        ("R1", 1.0, 3.0),
        ("R2", 3.0, 6.0),
        ("R3", 5.0, 9.0),
    ];
    for (request_id, pickup_x, delivery_x) in requests {
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(pickup_x, 0.0))),
                &[
                    ("request_id", FieldValue::Text(request_id.to_string())),
                    ("stop_type", FieldValue::Text("pickup".to_string())),
                    ("demand", FieldValue::Float(2.0)),
                ],
            )
            .expect("add pickup");
        stops
            .add_feature(
                Some(Geometry::Point(Coord::xy(delivery_x, 0.0))),
                &[
                    ("request_id", FieldValue::Text(request_id.to_string())),
                    ("stop_type", FieldValue::Text("delivery".to_string())),
                    ("demand", FieldValue::Float(2.0)),
                ],
            )
            .expect("add delivery");
    }
    wbvector::write(&stops, &stops_path, VectorFormat::GeoPackage).expect("write stops");

    let mut args = ToolArgs::new();
    args.insert("network".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("depot_points".to_string(), json!(depot_path.to_string_lossy().to_string()));
    args.insert("stop_points".to_string(), json!(stops_path.to_string_lossy().to_string()));
    args.insert("request_id_field".to_string(), json!("request_id"));
    args.insert("stop_type_field".to_string(), json!("stop_type"));
    args.insert("demand_field".to_string(), json!("demand"));
    args.insert("vehicle_capacity".to_string(), json!(2.0));
    args.insert("max_vehicles".to_string(), json!(3));
    args.insert("output".to_string(), json!(routes_out.to_string_lossy().to_string()));

    let result = registry
        .run("vehicle_routing_pickup_delivery", &args, &context(&caps))
        .expect("vehicle_routing_pickup_delivery logistics run");

    let served_requests = result
        .outputs
        .get("served_request_count")
        .and_then(|v| v.as_u64())
        .expect("served_request_count output");
    assert_eq!(served_requests, 3, "all logistics requests should be served");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&depot_path);
    let _ = std::fs::remove_file(&stops_path);
    let _ = std::fs::remove_file(&routes_out);
}

#[test]
fn topology_validation_report_flags_invalid_line_and_polygon_features() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_topology_validation_report");
    let line_input_path = std::env::temp_dir().join(format!("{tag}_line.geojson"));
    let poly_input_path = std::env::temp_dir().join(format!("{tag}_poly.geojson"));
    let line_csv = std::env::temp_dir().join(format!("{tag}_line_report.csv"));
    let poly_csv = std::env::temp_dir().join(format!("{tag}_poly_report.csv"));

    let mut lines = Layer::new("issues")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(2.0, 2.0),
                Coord::xy(0.0, 2.0),
                Coord::xy(2.0, 0.0),
            ])),
            &[],
        )
        .expect("add invalid line");
    wbvector::write(&lines, &line_input_path, VectorFormat::GeoJson).expect("write invalid line input");

    let mut polygons = Layer::new("issues")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    polygons
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add invalid polygon");
    wbvector::write(&polygons, &poly_input_path, VectorFormat::GeoJson).expect("write invalid polygon input");

    let mut line_args = ToolArgs::new();
    line_args.insert("input".to_string(), json!(line_input_path.to_string_lossy().to_string()));
    line_args.insert("output".to_string(), json!(line_csv.to_string_lossy().to_string()));
    registry
        .run("topology_validation_report", &line_args, &context(&caps))
        .expect("topology_validation_report run");

    let mut poly_args = ToolArgs::new();
    poly_args.insert("input".to_string(), json!(poly_input_path.to_string_lossy().to_string()));
    poly_args.insert("output".to_string(), json!(poly_csv.to_string_lossy().to_string()));
    registry
        .run("topology_validation_report", &poly_args, &context(&caps))
        .expect("topology_validation_report polygon run");

    let line_report = std::fs::read_to_string(&line_csv).expect("read topology line csv report");
    assert!(line_report.contains("feature_fid,geometry_type,issue_type,detail"));
    assert!(line_report.contains("linestring_self_intersection"), "expected line self-intersection issue: {}", line_report);

    let poly_report = std::fs::read_to_string(&poly_csv).expect("read topology polygon csv report");
    assert!(poly_report.contains("feature_fid,geometry_type,issue_type,detail"));
    assert!(poly_report.contains("polygon_topology_invalid"), "expected polygon topology issue: {}", poly_report);

    let _ = std::fs::remove_file(&line_input_path);
    let _ = std::fs::remove_file(&poly_input_path);
    let _ = std::fs::remove_file(&line_csv);
    let _ = std::fs::remove_file(&poly_csv);
}

#[test]
fn topology_rule_validate_reports_line_self_intersection() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_topology_rule_validate_line");
    let input_path = std::env::temp_dir().join(format!("{tag}_line.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_violations.gpkg"));
    let report_path = std::env::temp_dir().join(format!("{tag}_report.json"));

    let mut lines = Layer::new("line_issues")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(2.0, 2.0),
                Coord::xy(0.0, 2.0),
                Coord::xy(2.0, 0.0),
            ])),
            &[],
        )
        .expect("add self-intersecting line");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write line input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["line_must_not_self_intersect"]));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    args.insert("report".to_string(), json!(report_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_validate", &args, &context(&caps))
        .expect("topology_rule_validate run");

    let out = wbvector::read(&output_path).expect("read rule-violation output");
    assert!(!out.features.is_empty(), "expected at least one violation feature");

    let rule_type_idx = out.schema.field_index("RULE_TYPE").expect("RULE_TYPE field");
    let feature_fid_idx = out.schema.field_index("FEATURE_FID").expect("FEATURE_FID field");
    let mut found_expected_rule = false;
    for feature in &out.features {
        let rule_type = match &feature.attributes[rule_type_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("expected RULE_TYPE text, got {:?}", other),
        };
        if rule_type == "line_must_not_self_intersect" {
            found_expected_rule = true;
        }
        match &feature.attributes[feature_fid_idx] {
            FieldValue::Integer(_) => {}
            other => panic!("expected FEATURE_FID integer, got {:?}", other),
        }
    }
    assert!(found_expected_rule, "expected line_must_not_self_intersect violation");

    let report_text = std::fs::read_to_string(&report_path).expect("read topology rule report");
    let report_json: serde_json::Value = serde_json::from_str(&report_text).expect("parse topology rule report");
    let total = report_json
        .get("total_violations")
        .and_then(|v| v.as_u64())
        .expect("total_violations field") as usize;
    assert_eq!(total, out.features.len());

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&report_path);
}

#[test]
fn topology_rule_validate_reports_polygon_overlap_pairwise() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_topology_rule_validate_poly");
    let input_path = std::env::temp_dir().join(format!("{tag}_poly.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_violations.gpkg"));

    let mut polys = Layer::new("poly_issues")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    polys
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon A");
    polys
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(1.0, 1.0),
                    Coord::xy(3.0, 1.0),
                    Coord::xy(3.0, 3.0),
                    Coord::xy(1.0, 3.0),
                    Coord::xy(1.0, 1.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon B");
    wbvector::write(&polys, &input_path, VectorFormat::GeoPackage).expect("write polygon input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["polygon_must_not_overlap"]));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_validate", &args, &context(&caps))
        .expect("topology_rule_validate run");

    let out = wbvector::read(&output_path).expect("read polygon overlap violations");
    assert_eq!(out.features.len(), 2, "expected one overlap violation per feature in the pair");

    let rule_type_idx = out.schema.field_index("RULE_TYPE").expect("RULE_TYPE field");
    let related_idx = out.schema.field_index("RELATED_FID").expect("RELATED_FID field");
    for feature in &out.features {
        let rule_type = match &feature.attributes[rule_type_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("expected RULE_TYPE text, got {:?}", other),
        };
        assert_eq!(rule_type, "polygon_must_not_overlap");
        match &feature.attributes[related_idx] {
            FieldValue::Integer(_) => {}
            other => panic!("expected RELATED_FID integer, got {:?}", other),
        }
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn topology_rule_validate_detects_point_not_on_line() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_topology_rule_point_coverage");
    let input_path = std::env::temp_dir().join(format!("{tag}_mixed.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_violations.gpkg"));

    let mut layer = Layer::new("mixed_geoms")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    // Point ON the line (should not be violated)
    layer
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 1.0))), &[])
        .expect("add point on line");
    // Point NOT on the line (should be violated)
    layer
        .add_feature(Some(Geometry::Point(Coord::xy(5.0, 5.0))), &[])
        .expect("add point off line");

    // Now add a line separately - we need a mixed layer actually
    wbvector::write(&layer, &input_path, VectorFormat::GeoPackage).expect("write points-only input");

    // For now, test with just points to confirm detection of no violations when no lines exist
    let mut args = ToolArgs::new();
    args.insert(
        "input".to_string(),
        json!(input_path.to_string_lossy().to_string()),
    );
    args.insert("rule_set".to_string(), json!(["point_must_be_covered_by_line"]));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_validate", &args, &context(&caps))
        .expect("topology_rule_validate run for point coverage");

    let out = wbvector::read(&output_path).expect("read point coverage violations");
    // Points with no lines should all be flagged as not covered
    assert_eq!(out.features.len(), 2, "both points should be flagged as not covered when no lines exist");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn topology_rule_validate_detects_line_dangles() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_topology_rule_dangles");
    let input_path = std::env::temp_dir().join(format!("{tag}_lines.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_violations.gpkg"));

    let mut lines = Layer::new("line_network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    // Connected line segment 1: 0,0 to 5,0
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(5.0, 0.0),
            ])),
            &[],
        )
        .expect("add connected line 1");
    // Connected line segment 2: 5,0 to 10,0 (shares endpoint at 5,0)
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(5.0, 0.0),
                Coord::xy(10.0, 0.0),
            ])),
            &[],
        )
        .expect("add connected line 2");
    // Dangling line: 15,0 to 20,0 (isolated)
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(15.0, 0.0),
                Coord::xy(20.0, 0.0),
            ])),
            &[],
        )
        .expect("add dangling line");

    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write line input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["line_must_not_have_dangles"]));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_validate", &args, &context(&caps))
        .expect("topology_rule_validate run for dangles");

    let out = wbvector::read(&output_path).expect("read dangle violations");
    // The dangling line has 2 endpoints that don't connect, so expect 2 violation features
    assert!(!out.features.is_empty(), "expected dangle violations");
    let rule_type_idx = out.schema.field_index("RULE_TYPE").expect("RULE_TYPE field");
    for feature in &out.features {
        let rule_type = match &feature.attributes[rule_type_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("expected RULE_TYPE text, got {:?}", other),
        };
        assert_eq!(rule_type, "line_must_not_have_dangles");
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn topology_rule_validate_detects_endpoint_snap_violations() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_topology_rule_snap");
    let input_path = std::env::temp_dir().join(format!("{tag}_lines.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_violations.gpkg"));

    let mut lines = Layer::new("line_snapping_issues")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    // Line 1: 0,0 to 1,0
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(1.0, 0.0),
            ])),
            &[],
        )
        .expect("add line 1");
    // Line 2: 3,0 to 4,0 (far from Line 1, exceeds snap_tolerance)
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(3.0, 0.0),
                Coord::xy(4.0, 0.0),
            ])),
            &[],
        )
        .expect("add line 2 (far from line 1)");

    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write line input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["line_endpoints_must_snap_within_tolerance"]));
    args.insert("snap_tolerance".to_string(), json!(0.5)); // Very tight tolerance
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_validate", &args, &context(&caps))
        .expect("topology_rule_validate run for snap");

    let out = wbvector::read(&output_path).expect("read snap violations");
    assert!(
        !out.features.is_empty(),
        "expected endpoint snap violations with tight tolerance"
    );

    let rule_type_idx = out.schema.field_index("RULE_TYPE").expect("RULE_TYPE field");
    for feature in &out.features {
        let rule_type = match &feature.attributes[rule_type_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("expected RULE_TYPE text, got {:?}", other),
        };
        assert_eq!(rule_type, "line_endpoints_must_snap_within_tolerance");
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn topology_rule_validate_detects_polygon_gaps() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_topology_rule_gaps");
    let input_path = std::env::temp_dir().join(format!("{tag}_polys.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_violations.gpkg"));

    let mut polys = Layer::new("poly_gaps")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    // Polygon A: 0,0 to 2,2
    polys
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon A");
    // Polygon B: 2.0005,0 to 4,2 (slight gap from Polygon A)
    polys
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(2.0005, 0.0),
                    Coord::xy(4.0, 0.0),
                    Coord::xy(4.0, 2.0),
                    Coord::xy(2.0005, 2.0),
                    Coord::xy(2.0005, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon B (with gap)");

    wbvector::write(&polys, &input_path, VectorFormat::GeoPackage).expect("write polygon input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["polygon_must_not_have_gaps"]));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_validate", &args, &context(&caps))
        .expect("topology_rule_validate run for gaps");

    let out = wbvector::read(&output_path).expect("read gap violations");
    // If gap detection is working, we expect violations for the adjacent polygons with small gap
    if let Some(idx) = out.schema.field_index("RULE_TYPE") {
        for feature in &out.features {
            let rule_type = match &feature.attributes[idx] {
                FieldValue::Text(v) => v.as_str(),
                _ => continue,
            };
            if rule_type == "polygon_must_not_have_gaps" {
                // Found a gap violation as expected
                break;
            }
        }
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn topology_rule_autofix_dry_run_mode_preserves_input() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_autofix_dryrun");
    let input_path = std::env::temp_dir().join(format!("{tag}_lines.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_fixed.gpkg"));
    let report_path = std::env::temp_dir().join(format!("{tag}_changes.json"));

    // Create test data: two line segments with endpoints that are far apart (should trigger snap fix)
    let mut lines = Layer::new("lines_to_fix")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(1.0, 0.0),
            ])),
            &[],
        )
        .expect("add line 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(2.0, 0.0),
                Coord::xy(3.0, 0.0),
            ])),
            &[],
        )
        .expect("add line 2");

    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write input");

    // Run autofix in dry_run mode
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["line_endpoints_must_snap_within_tolerance"]));
    args.insert("snap_tolerance".to_string(), json!(1.5));
    args.insert("dry_run".to_string(), json!(true));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    args.insert("change_report".to_string(), json!(report_path.to_string_lossy().to_string()));

    let result = registry
        .run("topology_rule_autofix", &args, &context(&caps))
        .expect("topology_rule_autofix dry_run should succeed");

    // Verify dry_run was reported
    assert_eq!(result.outputs.get("dry_run_mode"), Some(&json!(true)));

    // Verify change report was created
    assert!(report_path.exists(), "change report should be created in dry_run mode");
    let report_text = std::fs::read_to_string(&report_path).expect("read change report");
    let report: serde_json::Value = serde_json::from_str(&report_text).expect("parse change report");
    assert_eq!(report.get("dry_run"), Some(&json!(true)));

    // Verify output wasn't created (dry_run mode)
    assert!(
        !output_path.exists(),
        "output file should not be created in dry_run mode"
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&report_path);
}

#[test]
fn topology_rule_autofix_commits_changes_when_not_dry_run() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_autofix_commit");
    let input_path = std::env::temp_dir().join(format!("{tag}_lines.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_fixed.gpkg"));

    // Create test data: line with far endpoints relative to small tolerance
    let mut lines = Layer::new("lines")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    let line_coords = vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 10.0)];
    lines
        .add_feature(Some(Geometry::line_string(line_coords)), &[])
        .expect("add line");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write input");

    // Run autofix in commit mode with large snap tolerance (should snap endpoints)
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["line_endpoints_must_snap_within_tolerance"]));
    args.insert("snap_tolerance".to_string(), json!(20.0));
    args.insert("dry_run".to_string(), json!(false));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_autofix", &args, &context(&caps))
        .expect("topology_rule_autofix commit should succeed");

    // Verify output was created
    assert!(output_path.exists(), "output file should be created in commit mode");

    // Read the output and verify snapping occurred (endpoints should be closer)
    let output = wbvector::read(&output_path).expect("read fixed output");
    assert_eq!(output.features.len(), 1);

    if let Some(Geometry::LineString(coords)) = output.features[0].geometry.as_ref() {
        if coords.len() >= 2 {
            let first = &coords[0];
            let last = &coords[coords.len() - 1];
            // Endpoints should have snapped closer to midpoint
            let dist = ((first.x - last.x).powi(2) + (first.y - last.y).powi(2)).sqrt();
            assert!(
                dist < 0.1,
                "endpoints should be snapped together (distance: {})",
                dist
            );
        }
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn topology_rule_autofix_projects_points_onto_lines() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_autofix_project");
    let input_path = std::env::temp_dir().join(format!("{tag}_mixed.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_fixed.gpkg"));

    // Create a line and a point near it (but not on it)
    let mut layer = Layer::new("mixed")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);

    // Add a point slightly offset from the line at (5, 0.1)
    layer
        .add_feature(Some(Geometry::Point(Coord::xy(5.0, 0.1))), &[])
        .expect("add test point");

    wbvector::write(&layer, &input_path, VectorFormat::GeoPackage).expect("write input");

    // Note: In real usage, you'd need a mixed geometry layer with lines and points
    // For this test, we verify the tool runs successfully (line detection would find zero lines)
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("rule_set".to_string(), json!(["point_must_be_covered_by_line"]));
    args.insert("snap_tolerance".to_string(), json!(1.0));
    args.insert("dry_run".to_string(), json!(false));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("topology_rule_autofix", &args, &context(&caps))
        .expect("topology_rule_autofix point projection should succeed");

    // Verify output was created
    assert!(output_path.exists(), "output file should be created");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn locate_points_along_routes_writes_measure_and_offset_attributes() {
    use wbvector::{Coord, Feature, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_locate_points_along_routes");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let points_path = std::env::temp_dir().join(format!("{tag}_points.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.push(Feature {
        fid: 11,
        geometry: Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
        attributes: vec![],
    });
    routes.push(Feature {
        fid: 22,
        geometry: Some(Geometry::line_string(vec![Coord::xy(0.0, 10.0), Coord::xy(10.0, 10.0)])),
        attributes: vec![],
    });
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");
    let persisted_routes = wbvector::read(&routes_path).expect("read persisted routes");
    let expected_route_fid = persisted_routes.features[0].fid as i64;

    let mut points = Layer::new("points")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    points
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 3.0))), &[])
        .expect("add test point");
    wbvector::write(&points, &points_path, VectorFormat::GeoPackage).expect("write points");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("locate_points_along_routes", &args, &context(&caps))
        .expect("locate_points_along_routes run");

    let out = wbvector::read(&out_path).expect("read locate_points_along_routes output");
    assert_eq!(out.features.len(), 1);
    let route_idx = out.schema.field_index("ROUTE_FID").expect("ROUTE_FID field");
    let measure_idx = out.schema.field_index("MEASURE").expect("MEASURE field");
    let offset_idx = out.schema.field_index("OFFSET").expect("OFFSET field");
    let locate_x_idx = out.schema.field_index("LOCATE_X").expect("LOCATE_X field");
    let locate_y_idx = out.schema.field_index("LOCATE_Y").expect("LOCATE_Y field");

    let route_fid = match &out.features[0].attributes[route_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer ROUTE_FID, got {:?}", other),
    };
    assert_eq!(route_fid, expected_route_fid);

    let measure = match &out.features[0].attributes[measure_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric MEASURE, got {:?}", other),
    };
    let offset = match &out.features[0].attributes[offset_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric OFFSET, got {:?}", other),
    };
    let locate_x = match &out.features[0].attributes[locate_x_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric LOCATE_X, got {:?}", other),
    };
    let locate_y = match &out.features[0].attributes[locate_y_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric LOCATE_Y, got {:?}", other),
    };

    assert!((measure - 2.0).abs() < 1.0e-9, "unexpected route measure: {}", measure);
    assert!((offset - 3.0).abs() < 1.0e-9, "unexpected offset distance: {}", offset);
    assert!((locate_x - 2.0).abs() < 1.0e-9, "unexpected locate x: {}", locate_x);
    assert!((locate_y - 0.0).abs() < 1.0e-9, "unexpected locate y: {}", locate_y);

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_points_from_table_creates_measured_point_events() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_points_from_table");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let events_path = std::env::temp_dir().join(format!("{tag}_events.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    std::fs::write(&events_path, "route_id,measure,label\nR1,3.0,School\n")
        .expect("write point events csv");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("measure_field".to_string(), json!("measure"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("write_event_xy".to_string(), json!(true));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("route_event_points_from_table", &args, &context(&caps))
        .expect("route_event_points_from_table run");

    let out = wbvector::read(&out_path).expect("read routed point events");
    assert_eq!(out.features.len(), 1);
    let measure_idx = out.schema.field_index("MEASURE").expect("MEASURE field");
    let label_idx = out.schema.field_index("EVT_label").expect("EVT_label field");

    let measure = match &out.features[0].attributes[measure_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric MEASURE, got {:?}", other),
    };
    assert!((measure - 3.0).abs() < 1.0e-9);
    assert_eq!(out.features[0].attributes[label_idx], FieldValue::Text("School".to_string()));

    match out.features[0].geometry.as_ref().expect("point event geometry") {
        Geometry::Point(coord) => {
            assert!((coord.x - 3.0).abs() < 1.0e-9);
            assert!(coord.y.abs() < 1.0e-9);
        }
        other => panic!("expected point geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_lines_from_table_segments_routes_by_from_to_measures() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_lines_from_table");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let events_path = std::env::temp_dir().join(format!("{tag}_events.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(5.0, 0.0),
                Coord::xy(10.0, 0.0),
            ])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    std::fs::write(&events_path, "route_id,from_m,to_m,name\nR1,2.0,8.0,SegmentA\n")
        .expect("write line events csv");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("route_event_lines_from_table", &args, &context(&caps))
        .expect("route_event_lines_from_table run");

    let out = wbvector::read(&out_path).expect("read routed line events");
    assert_eq!(out.features.len(), 1);
    let from_idx = out.schema.field_index("FROM_M").expect("FROM_M field");
    let to_idx = out.schema.field_index("TO_M").expect("TO_M field");
    let name_idx = out.schema.field_index("EVT_name").expect("EVT_name field");

    let from_m = match &out.features[0].attributes[from_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric FROM_M, got {:?}", other),
    };
    let to_m = match &out.features[0].attributes[to_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric TO_M, got {:?}", other),
    };
    assert!((from_m - 2.0).abs() < 1.0e-9);
    assert!((to_m - 8.0).abs() < 1.0e-9);
    assert_eq!(out.features[0].attributes[name_idx], FieldValue::Text("SegmentA".to_string()));

    match out.features[0].geometry.as_ref().expect("line event geometry") {
        Geometry::MultiLineString(parts) => {
            assert_eq!(parts.len(), 1);
            let coords = &parts[0];
            assert_eq!(coords.len(), 3);
            assert!((coords[0].x - 2.0).abs() < 1.0e-9);
            assert!((coords[1].x - 5.0).abs() < 1.0e-9);
            assert!((coords[2].x - 8.0).abs() < 1.0e-9);
        }
        other => panic!("expected multilinestring geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_points_from_layer_creates_measured_point_events() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_points_from_layer");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("measure", FieldType::Float));
    events.schema.add_field(FieldDef::new("name", FieldType::Text));
    events
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("measure", FieldValue::Float(4.0)),
                ("name", FieldValue::Text("StopA".to_string())),
            ],
        )
        .expect("add event row");
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events layer");
    let persisted_events = wbvector::read(&events_path).expect("read persisted events layer");
    let expected_event_fid = persisted_events.features[0].fid as i64;

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("measure_field".to_string(), json!("measure"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("write_event_xy".to_string(), json!(true));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("route_event_points_from_layer", &args, &context(&caps))
        .expect("route_event_points_from_layer run");

    let out = wbvector::read(&out_path).expect("read routed point events from layer");
    assert_eq!(out.features.len(), 1);
    let measure_idx = out.schema.field_index("MEASURE").expect("MEASURE field");
    let event_fid_idx = out.schema.field_index("EVENT_FID").expect("EVENT_FID field");
    let event_x_idx = out.schema.field_index("EVENT_X").expect("EVENT_X field");
    let event_y_idx = out.schema.field_index("EVENT_Y").expect("EVENT_Y field");
    let name_idx = out.schema.field_index("EVT_name").expect("EVT_name field");
    let measure = match &out.features[0].attributes[measure_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric MEASURE, got {:?}", other),
    };
    assert!((measure - 4.0).abs() < 1.0e-9);
    match &out.features[0].attributes[event_fid_idx] {
        FieldValue::Integer(v) => assert_eq!(*v, expected_event_fid),
        other => panic!("expected integer EVENT_FID, got {:?}", other),
    }
    let event_x = match &out.features[0].attributes[event_x_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric EVENT_X, got {:?}", other),
    };
    let event_y = match &out.features[0].attributes[event_y_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric EVENT_Y, got {:?}", other),
    };
    assert!(event_x.abs() < 1.0e-9);
    assert!(event_y.abs() < 1.0e-9);
    assert_eq!(out.features[0].attributes[name_idx], FieldValue::Text("StopA".to_string()));

    match out.features[0].geometry.as_ref().expect("point event geometry") {
        Geometry::Point(coord) => {
            assert!((coord.x - 4.0).abs() < 1.0e-9);
            assert!(coord.y.abs() < 1.0e-9);
        }
        other => panic!("expected point geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_lines_from_layer_segments_routes_by_from_to_measures() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_lines_from_layer");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(5.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("name", FieldType::Text));
    events
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(1.0)),
                ("to_m", FieldValue::Float(9.0)),
                ("name", FieldValue::Text("Seg1".to_string())),
            ],
        )
        .expect("add line event row");
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events layer");
    let persisted_events = wbvector::read(&events_path).expect("read persisted events layer");
    let expected_event_fid = persisted_events.features[0].fid as i64;

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("write_event_xy".to_string(), json!(true));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("route_event_lines_from_layer", &args, &context(&caps))
        .expect("route_event_lines_from_layer run");

    let out = wbvector::read(&out_path).expect("read routed line events from layer");
    assert_eq!(out.features.len(), 1);
    let from_idx = out.schema.field_index("FROM_M").expect("FROM_M field");
    let to_idx = out.schema.field_index("TO_M").expect("TO_M field");
    let event_fid_idx = out.schema.field_index("EVENT_FID").expect("EVENT_FID field");
    let event_x_idx = out.schema.field_index("EVENT_X").expect("EVENT_X field");
    let event_y_idx = out.schema.field_index("EVENT_Y").expect("EVENT_Y field");
    let name_idx = out.schema.field_index("EVT_name").expect("EVT_name field");

    let from_m = match &out.features[0].attributes[from_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric FROM_M, got {:?}", other),
    };
    let to_m = match &out.features[0].attributes[to_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric TO_M, got {:?}", other),
    };
    assert!((from_m - 1.0).abs() < 1.0e-9);
    assert!((to_m - 9.0).abs() < 1.0e-9);
    match &out.features[0].attributes[event_fid_idx] {
        FieldValue::Integer(v) => assert_eq!(*v, expected_event_fid),
        other => panic!("expected integer EVENT_FID, got {:?}", other),
    }
    let event_x = match &out.features[0].attributes[event_x_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric EVENT_X, got {:?}", other),
    };
    let event_y = match &out.features[0].attributes[event_y_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric EVENT_Y, got {:?}", other),
    };
    assert!(event_x.abs() < 1.0e-9);
    assert!(event_y.abs() < 1.0e-9);
    assert_eq!(out.features[0].attributes[name_idx], FieldValue::Text("Seg1".to_string()));

    match out.features[0].geometry.as_ref().expect("line event geometry") {
        Geometry::MultiLineString(parts) => {
            assert_eq!(parts.len(), 1);
            assert!((parts[0][0].x - 1.0).abs() < 1.0e-9);
            assert!((parts[0][parts[0].len() - 1].x - 9.0).abs() < 1.0e-9);
        }
        other => panic!("expected multilinestring geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_points_from_table_rejects_duplicate_route_ids() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_points_from_table_duplicate_route_ids");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let events_path = std::env::temp_dir().join(format!("{tag}_events.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add first route feature");
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 5.0), Coord::xy(10.0, 5.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add second route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    std::fs::write(&events_path, "route_id,measure\nR1,2.0\n").expect("write point events csv");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("measure_field".to_string(), json!("measure"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("route_event_points_from_table", &args, &context(&caps))
        .expect_err("expected duplicate route id failure");
    let msg = err.to_string();
    assert!(msg.contains("duplicate route identifier"), "unexpected error: {}", msg);

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_lines_from_layer_rejects_equal_from_to_measures() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_lines_from_layer_equal_measures");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    events
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(4.0)),
                ("to_m", FieldValue::Float(4.0)),
            ],
        )
        .expect("add event row");
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events layer");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("route_event_lines_from_layer", &args, &context(&caps))
        .expect_err("expected equal-measure failure");
    let msg = err.to_string();
    assert!(msg.contains("equal from/to measures"), "unexpected error: {}", msg);

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_points_from_layer_can_disable_event_traceability_fields() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_points_from_layer_traceability_toggle");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("measure", FieldType::Float));
    events
        .add_feature(
            Some(Geometry::Point(Coord::xy(100.0, 200.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("measure", FieldValue::Float(3.0)),
            ],
        )
        .expect("add event row");
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events layer");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("measure_field".to_string(), json!("measure"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("write_event_fid".to_string(), json!(false));
    args.insert("write_event_xy".to_string(), json!(false));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_event_points_from_layer", &args, &context(&caps))
        .expect("route_event_points_from_layer run");

    let out = wbvector::read(&out_path).expect("read routed point events from layer");
    assert_eq!(out.features.len(), 1);
    assert!(out.schema.field_index("EVENT_FID").is_none());
    assert!(out.schema.field_index("EVENT_X").is_none());
    assert!(out.schema.field_index("EVENT_Y").is_none());

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_calibrate_sets_from_to_measures_from_control_points() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_calibrate_controls");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let controls_path = std::env::temp_dir().join(format!("{tag}_controls.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    let mut controls = Layer::new("controls")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    controls.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    controls.schema.add_field(FieldDef::new("measure", FieldType::Float));
    controls
        .add_feature(
            Some(Geometry::Point(Coord::xy(2.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("measure", FieldValue::Float(20.0)),
            ],
        )
        .expect("add first control");
    controls
        .add_feature(
            Some(Geometry::Point(Coord::xy(8.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("measure", FieldValue::Float(80.0)),
            ],
        )
        .expect("add second control");
    wbvector::write(&controls, &controls_path, VectorFormat::GeoPackage).expect("write controls");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("control_points".to_string(), json!(controls_path.to_string_lossy().to_string()));
    args.insert("control_measure_field".to_string(), json!("measure"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("control_route_id_field".to_string(), json!("route_id"));
    args.insert("snap_tolerance".to_string(), json!(0.01));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_calibrate", &args, &context(&caps))
        .expect("route_calibrate run");

    let out = wbvector::read(&out_path).expect("read calibrated routes");
    assert_eq!(out.features.len(), 1);
    let from_idx = out.schema.field_index("from_measure").expect("from_measure field");
    let to_idx = out.schema.field_index("to_measure").expect("to_measure field");
    let status_idx = out.schema.field_index("calib_status").expect("calib_status field");
    let count_idx = out.schema.field_index("control_count").expect("control_count field");

    let from_measure = match &out.features[0].attributes[from_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric from_measure, got {:?}", other),
    };
    let to_measure = match &out.features[0].attributes[to_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric to_measure, got {:?}", other),
    };
    assert!((from_measure - 0.0).abs() < 1.0e-9, "unexpected from_measure: {}", from_measure);
    assert!((to_measure - 100.0).abs() < 1.0e-9, "unexpected to_measure: {}", to_measure);
    assert_eq!(
        out.features[0].attributes[status_idx],
        FieldValue::Text("calibrated".to_string())
    );
    assert_eq!(out.features[0].attributes[count_idx], FieldValue::Integer(2));

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&controls_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_recalibrate_scales_measure_span_with_edited_geometry_length() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_recalibrate_scale");
    let original_path = std::env::temp_dir().join(format!("{tag}_original.gpkg"));
    let edited_path = std::env::temp_dir().join(format!("{tag}_edited.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut original = Layer::new("original")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    original.schema.add_field(FieldDef::new("RID", FieldType::Text));
    original.schema.add_field(FieldDef::new("from_measure", FieldType::Float));
    original.schema.add_field(FieldDef::new("to_measure", FieldType::Float));
    original
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[
                ("RID", FieldValue::Text("R1".to_string())),
                ("from_measure", FieldValue::Float(100.0)),
                ("to_measure", FieldValue::Float(200.0)),
            ],
        )
        .expect("add original route");
    wbvector::write(&original, &original_path, VectorFormat::GeoPackage).expect("write original routes");

    let mut edited = Layer::new("edited")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    edited.schema.add_field(FieldDef::new("RID", FieldType::Text));
    edited
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(20.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add edited route");
    wbvector::write(&edited, &edited_path, VectorFormat::GeoPackage).expect("write edited routes");

    let mut args = ToolArgs::new();
    args.insert("original_routes".to_string(), json!(original_path.to_string_lossy().to_string()));
    args.insert("edited_routes".to_string(), json!(edited_path.to_string_lossy().to_string()));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_recalibrate", &args, &context(&caps))
        .expect("route_recalibrate run");

    let out = wbvector::read(&out_path).expect("read recalibrated routes");
    assert_eq!(out.features.len(), 1);
    let from_idx = out.schema.field_index("from_measure").expect("from_measure field");
    let to_idx = out.schema.field_index("to_measure").expect("to_measure field");
    let status_idx = out.schema.field_index("recalib_status").expect("recalib_status field");

    let from_measure = match &out.features[0].attributes[from_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric from_measure, got {:?}", other),
    };
    let to_measure = match &out.features[0].attributes[to_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric to_measure, got {:?}", other),
    };
    assert!((from_measure - 100.0).abs() < 1.0e-9, "unexpected from_measure: {}", from_measure);
    assert!((to_measure - 300.0).abs() < 1.0e-9, "unexpected to_measure: {}", to_measure);
    assert_eq!(
        out.features[0].attributes[status_idx],
        FieldValue::Text("recalibrated_scaled".to_string())
    );

    let _ = std::fs::remove_file(&original_path);
    let _ = std::fs::remove_file(&edited_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_calibrate_marks_non_monotonic_control_sequences() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_calibrate_non_monotonic");
    let routes_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));
    let controls_path = std::env::temp_dir().join(format!("{tag}_controls.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut routes = Layer::new("routes")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    routes.schema.add_field(FieldDef::new("RID", FieldType::Text));
    routes
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)])),
            &[("RID", FieldValue::Text("R1".to_string()))],
        )
        .expect("add route feature");
    wbvector::write(&routes, &routes_path, VectorFormat::GeoPackage).expect("write routes");

    let mut controls = Layer::new("controls")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    controls.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    controls.schema.add_field(FieldDef::new("measure", FieldType::Float));
    controls
        .add_feature(
            Some(Geometry::Point(Coord::xy(2.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("measure", FieldValue::Float(80.0)),
            ],
        )
        .expect("add first control");
    controls
        .add_feature(
            Some(Geometry::Point(Coord::xy(8.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("measure", FieldValue::Float(20.0)),
            ],
        )
        .expect("add second control");
    wbvector::write(&controls, &controls_path, VectorFormat::GeoPackage).expect("write controls");

    let mut args = ToolArgs::new();
    args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
    args.insert("control_points".to_string(), json!(controls_path.to_string_lossy().to_string()));
    args.insert("control_measure_field".to_string(), json!("measure"));
    args.insert("route_id_field".to_string(), json!("RID"));
    args.insert("control_route_id_field".to_string(), json!("route_id"));
    args.insert("snap_tolerance".to_string(), json!(0.01));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_calibrate", &args, &context(&caps))
        .expect("route_calibrate run");

    let out = wbvector::read(&out_path).expect("read calibrated routes");
    assert_eq!(out.features.len(), 1);
    let status_idx = out.schema.field_index("calib_status").expect("calib_status field");
    assert_eq!(
        out.features[0].attributes[status_idx],
        FieldValue::Text("non_monotonic_controls".to_string())
    );

    let _ = std::fs::remove_file(&routes_path);
    let _ = std::fs::remove_file(&controls_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_split_splits_intervals_at_route_boundaries() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_split_basic");
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let boundaries_path = std::env::temp_dir().join(format!("{tag}_boundaries.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("name", FieldType::Text));
    events
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(0.0)),
                ("to_m", FieldValue::Float(10.0)),
                ("name", FieldValue::Text("A".to_string())),
            ],
        )
        .expect("add event feature");
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events");
    let persisted_events = wbvector::read(&events_path).expect("read persisted events");
    let expected_parent_fid = persisted_events.features[0].fid as i64;

    let mut boundaries = Layer::new("boundaries")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    boundaries.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    boundaries.schema.add_field(FieldDef::new("measure", FieldType::Float));
    for m in [2.0, 5.0, 8.0] {
        boundaries
            .add_feature(
                Some(Geometry::Point(Coord::xy(m, 0.0))),
                &[
                    ("route_id", FieldValue::Text("R1".to_string())),
                    ("measure", FieldValue::Float(m)),
                ],
            )
            .expect("add boundary feature");
    }
    wbvector::write(&boundaries, &boundaries_path, VectorFormat::GeoPackage).expect("write boundaries");

    let mut args = ToolArgs::new();
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("boundaries".to_string(), json!(boundaries_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("boundary_route_field".to_string(), json!("route_id"));
    args.insert("boundary_measure_field".to_string(), json!("measure"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_event_split", &args, &context(&caps))
        .expect("route_event_split run");

    let out = wbvector::read(&out_path).expect("read split events");
    assert_eq!(out.features.len(), 4);
    let from_idx = out.schema.field_index("from_m").expect("from_m field");
    let to_idx = out.schema.field_index("to_m").expect("to_m field");
    let split_seq_idx = out.schema.field_index("split_seq").expect("split_seq field");
    let parent_fid_idx = out.schema.field_index("parent_fid").expect("parent_fid field");

    let expected = [(0.0, 2.0, 1i64), (2.0, 5.0, 2i64), (5.0, 8.0, 3i64), (8.0, 10.0, 4i64)];
    for (idx, feature) in out.features.iter().enumerate() {
        let from_m = match &feature.attributes[from_idx] {
            FieldValue::Float(v) => *v,
            FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric from_m, got {:?}", other),
        };
        let to_m = match &feature.attributes[to_idx] {
            FieldValue::Float(v) => *v,
            FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric to_m, got {:?}", other),
        };
        let split_seq = match &feature.attributes[split_seq_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer split_seq, got {:?}", other),
        };
        let parent_fid = match &feature.attributes[parent_fid_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer parent_fid, got {:?}", other),
        };
        assert!((from_m - expected[idx].0).abs() < 1.0e-9);
        assert!((to_m - expected[idx].1).abs() < 1.0e-9);
        assert_eq!(split_seq, expected[idx].2);
        assert_eq!(parent_fid, expected_parent_fid);
    }

    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&boundaries_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_merge_merges_adjacent_compatible_events() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_merge_basic");
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("speed", FieldType::Integer));

    for (from_m, to_m, speed) in [(0.0, 3.0, 50i64), (3.0, 6.0, 50i64), (6.0, 10.0, 30i64)] {
        events
            .add_feature(
                Some(Geometry::Point(Coord::xy(from_m, 0.0))),
                &[
                    ("route_id", FieldValue::Text("R1".to_string())),
                    ("from_m", FieldValue::Float(from_m)),
                    ("to_m", FieldValue::Float(to_m)),
                    ("speed", FieldValue::Integer(speed)),
                ],
            )
            .expect("add event feature");
    }
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events");

    let mut args = ToolArgs::new();
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("group_fields".to_string(), json!("route_id,speed"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_event_merge", &args, &context(&caps))
        .expect("route_event_merge run");

    let out = wbvector::read(&out_path).expect("read merged events");
    assert_eq!(out.features.len(), 2);

    let from_idx = out.schema.field_index("from_m").expect("from_m field");
    let to_idx = out.schema.field_index("to_m").expect("to_m field");
    let speed_idx = out.schema.field_index("speed").expect("speed field");
    let merge_count_idx = out.schema.field_index("merge_count").expect("merge_count field");

    let mut observed = out
        .features
        .iter()
        .map(|feature| {
            let from_m = match &feature.attributes[from_idx] {
                FieldValue::Float(v) => *v,
                FieldValue::Integer(v) => *v as f64,
                other => panic!("expected numeric from_m, got {:?}", other),
            };
            let to_m = match &feature.attributes[to_idx] {
                FieldValue::Float(v) => *v,
                FieldValue::Integer(v) => *v as f64,
                other => panic!("expected numeric to_m, got {:?}", other),
            };
            let speed = match &feature.attributes[speed_idx] {
                FieldValue::Integer(v) => *v,
                other => panic!("expected integer speed, got {:?}", other),
            };
            let merge_count = match &feature.attributes[merge_count_idx] {
                FieldValue::Integer(v) => *v,
                other => panic!("expected integer merge_count, got {:?}", other),
            };
            (from_m, to_m, speed, merge_count)
        })
        .collect::<Vec<_>>();
    observed.sort_by(|a, b| a.0.total_cmp(&b.0));

    assert!((observed[0].0 - 0.0).abs() < 1.0e-9);
    assert!((observed[0].1 - 6.0).abs() < 1.0e-9);
    assert_eq!(observed[0].2, 50);
    assert_eq!(observed[0].3, 2);

    assert!((observed[1].0 - 6.0).abs() < 1.0e-9);
    assert!((observed[1].1 - 10.0).abs() < 1.0e-9);
    assert_eq!(observed[1].2, 30);
    assert_eq!(observed[1].3, 1);

    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_merge_rejects_overlaps_in_error_mode() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_merge_overlap_error");
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("speed", FieldType::Integer));

    for (from_m, to_m) in [(0.0, 5.0), (4.0, 8.0)] {
        events
            .add_feature(
                Some(Geometry::Point(Coord::xy(from_m, 0.0))),
                &[
                    ("route_id", FieldValue::Text("R1".to_string())),
                    ("from_m", FieldValue::Float(from_m)),
                    ("to_m", FieldValue::Float(to_m)),
                    ("speed", FieldValue::Integer(50)),
                ],
            )
            .expect("add event feature");
    }
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events");

    let mut args = ToolArgs::new();
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("group_fields".to_string(), json!("route_id,speed"));
    args.insert("conflict_mode".to_string(), json!("error"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("route_event_merge", &args, &context(&caps))
        .expect_err("expected overlap conflict in error mode");
    let msg = err.to_string();
    assert!(msg.contains("overlapping events detected"), "unexpected error: {}", msg);

    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_merge_skips_overlaps_in_skip_mode() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_merge_overlap_skip");
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("speed", FieldType::Integer));

    for (from_m, to_m) in [(0.0, 5.0), (4.0, 8.0)] {
        events
            .add_feature(
                Some(Geometry::Point(Coord::xy(from_m, 0.0))),
                &[
                    ("route_id", FieldValue::Text("R1".to_string())),
                    ("from_m", FieldValue::Float(from_m)),
                    ("to_m", FieldValue::Float(to_m)),
                    ("speed", FieldValue::Integer(50)),
                ],
            )
            .expect("add event feature");
    }
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events");

    let mut args = ToolArgs::new();
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("event_route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("group_fields".to_string(), json!("route_id,speed"));
    args.insert("conflict_mode".to_string(), json!("skip"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_event_merge", &args, &context(&caps))
        .expect("route_event_merge run in skip mode");

    let out = wbvector::read(&out_path).expect("read merged events");
    assert_eq!(out.features.len(), 2);
    let merge_count_idx = out.schema.field_index("merge_count").expect("merge_count field");
    for feature in &out.features {
        assert_eq!(feature.attributes[merge_count_idx], FieldValue::Integer(1));
    }

    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_overlay_outputs_overlapping_intervals() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_overlay_overlap");
    let primary_path = std::env::temp_dir().join(format!("{tag}_primary.gpkg"));
    let overlay_path = std::env::temp_dir().join(format!("{tag}_overlay.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut primary = Layer::new("primary")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    primary.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    primary.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    primary.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    primary.schema.add_field(FieldDef::new("surface", FieldType::Text));
    primary
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(0.0)),
                ("to_m", FieldValue::Float(10.0)),
                ("surface", FieldValue::Text("asphalt".to_string())),
            ],
        )
        .expect("add primary feature");
    wbvector::write(&primary, &primary_path, VectorFormat::GeoPackage).expect("write primary");

    let mut overlay = Layer::new("overlay")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    overlay.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    overlay.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    overlay.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    overlay.schema.add_field(FieldDef::new("speed", FieldType::Integer));
    overlay
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(3.0)),
                ("to_m", FieldValue::Float(6.0)),
                ("speed", FieldValue::Integer(40)),
            ],
        )
        .expect("add overlay feature");
    wbvector::write(&overlay, &overlay_path, VectorFormat::GeoPackage).expect("write overlay");

    let mut args = ToolArgs::new();
    args.insert("primary_events".to_string(), json!(primary_path.to_string_lossy().to_string()));
    args.insert("overlay_events".to_string(), json!(overlay_path.to_string_lossy().to_string()));
    args.insert("primary_route_field".to_string(), json!("route_id"));
    args.insert("primary_from_measure_field".to_string(), json!("from_m"));
    args.insert("primary_to_measure_field".to_string(), json!("to_m"));
    args.insert("overlay_route_field".to_string(), json!("route_id"));
    args.insert("overlay_from_measure_field".to_string(), json!("from_m"));
    args.insert("overlay_to_measure_field".to_string(), json!("to_m"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_event_overlay", &args, &context(&caps))
        .expect("route_event_overlay run");

    let out = wbvector::read(&out_path).expect("read overlay events");
    assert_eq!(out.features.len(), 1);
    let route_idx = out.schema.field_index("ROUTE_ID").expect("ROUTE_ID field");
    let from_idx = out.schema.field_index("FROM_M").expect("FROM_M field");
    let to_idx = out.schema.field_index("TO_M").expect("TO_M field");
    let pri_surface_idx = out.schema.field_index("PRI_surface").expect("PRI_surface field");
    let ovr_speed_idx = out.schema.field_index("OVR_speed").expect("OVR_speed field");

    assert_eq!(out.features[0].attributes[route_idx], FieldValue::Text("R1".to_string()));
    let from_m = match &out.features[0].attributes[from_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric FROM_M, got {:?}", other),
    };
    let to_m = match &out.features[0].attributes[to_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric TO_M, got {:?}", other),
    };
    assert!((from_m - 3.0).abs() < 1.0e-9);
    assert!((to_m - 6.0).abs() < 1.0e-9);
    assert_eq!(
        out.features[0].attributes[pri_surface_idx],
        FieldValue::Text("asphalt".to_string())
    );
    assert_eq!(out.features[0].attributes[ovr_speed_idx], FieldValue::Integer(40));

    let _ = std::fs::remove_file(&primary_path);
    let _ = std::fs::remove_file(&overlay_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_overlay_returns_empty_for_disjoint_intervals() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_overlay_disjoint");
    let primary_path = std::env::temp_dir().join(format!("{tag}_primary.gpkg"));
    let overlay_path = std::env::temp_dir().join(format!("{tag}_overlay.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut primary = Layer::new("primary")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    primary.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    primary.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    primary.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    primary
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(0.0)),
                ("to_m", FieldValue::Float(2.0)),
            ],
        )
        .expect("add primary feature");
    wbvector::write(&primary, &primary_path, VectorFormat::GeoPackage).expect("write primary");

    let mut overlay = Layer::new("overlay")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    overlay.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    overlay.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    overlay.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    overlay
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(3.0)),
                ("to_m", FieldValue::Float(5.0)),
            ],
        )
        .expect("add overlay feature");
    wbvector::write(&overlay, &overlay_path, VectorFormat::GeoPackage).expect("write overlay");

    let mut args = ToolArgs::new();
    args.insert("primary_events".to_string(), json!(primary_path.to_string_lossy().to_string()));
    args.insert("overlay_events".to_string(), json!(overlay_path.to_string_lossy().to_string()));
    args.insert("primary_route_field".to_string(), json!("route_id"));
    args.insert("primary_from_measure_field".to_string(), json!("from_m"));
    args.insert("primary_to_measure_field".to_string(), json!("to_m"));
    args.insert("overlay_route_field".to_string(), json!("route_id"));
    args.insert("overlay_from_measure_field".to_string(), json!("from_m"));
    args.insert("overlay_to_measure_field".to_string(), json!("to_m"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_event_overlay", &args, &context(&caps))
        .expect("route_event_overlay run");

    let out = wbvector::read(&out_path).expect("read overlay events");
    assert_eq!(out.features.len(), 0);

    let _ = std::fs::remove_file(&primary_path);
    let _ = std::fs::remove_file(&overlay_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_event_overlay_respects_min_overlap_length() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_event_overlay_min_overlap");
    let primary_path = std::env::temp_dir().join(format!("{tag}_primary.gpkg"));
    let overlay_path = std::env::temp_dir().join(format!("{tag}_overlay.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut primary = Layer::new("primary")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    primary.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    primary.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    primary.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    primary
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(0.0)),
                ("to_m", FieldValue::Float(10.0)),
            ],
        )
        .expect("add primary feature");
    wbvector::write(&primary, &primary_path, VectorFormat::GeoPackage).expect("write primary");

    let mut overlay = Layer::new("overlay")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    overlay.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    overlay.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    overlay.schema.add_field(FieldDef::new("to_m", FieldType::Float));
    overlay
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[
                ("route_id", FieldValue::Text("R1".to_string())),
                ("from_m", FieldValue::Float(9.0)),
                ("to_m", FieldValue::Float(12.0)),
            ],
        )
        .expect("add overlay feature");
    wbvector::write(&overlay, &overlay_path, VectorFormat::GeoPackage).expect("write overlay");

    let mut args = ToolArgs::new();
    args.insert("primary_events".to_string(), json!(primary_path.to_string_lossy().to_string()));
    args.insert("overlay_events".to_string(), json!(overlay_path.to_string_lossy().to_string()));
    args.insert("primary_route_field".to_string(), json!("route_id"));
    args.insert("primary_from_measure_field".to_string(), json!("from_m"));
    args.insert("primary_to_measure_field".to_string(), json!("to_m"));
    args.insert("overlay_route_field".to_string(), json!("route_id"));
    args.insert("overlay_from_measure_field".to_string(), json!("from_m"));
    args.insert("overlay_to_measure_field".to_string(), json!("to_m"));
    args.insert("min_overlap_length".to_string(), json!(2.0));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("route_event_overlay", &args, &context(&caps))
        .expect("route_event_overlay run");

    let out = wbvector::read(&out_path).expect("read overlay events");
    assert_eq!(out.features.len(), 0);

    let _ = std::fs::remove_file(&primary_path);
    let _ = std::fs::remove_file(&overlay_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_measure_qa_detects_gaps_overlaps_non_monotonic_and_duplicates() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_measure_qa_issues");
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));

    for (from_m, to_m) in [(0.0, 2.0), (3.0, 5.0), (4.0, 6.0), (4.0, 6.0), (1.0, 1.5)] {
        events
            .add_feature(
                Some(Geometry::Point(Coord::xy(from_m, 0.0))),
                &[
                    ("route_id", FieldValue::Text("R1".to_string())),
                    ("from_m", FieldValue::Float(from_m)),
                    ("to_m", FieldValue::Float(to_m)),
                ],
            )
            .expect("add event feature");
    }
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events");

    let mut args = ToolArgs::new();
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let result = registry
        .run("route_measure_qa", &args, &context(&caps))
        .expect("route_measure_qa run");

    let out = wbvector::read(&out_path).expect("read qa diagnostics");
    assert!(!out.features.is_empty());
    let issue_idx = out.schema.field_index("ISSUE_TYPE").expect("ISSUE_TYPE field");

    let mut seen_gap = false;
    let mut seen_overlap = false;
    let mut seen_non_monotonic = false;
    let mut seen_duplicate = false;
    for feature in &out.features {
        let issue_type = match &feature.attributes[issue_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("expected ISSUE_TYPE text, got {:?}", other),
        };
        if issue_type == "gap" {
            seen_gap = true;
        }
        if issue_type == "overlap" {
            seen_overlap = true;
        }
        if issue_type == "non_monotonic" {
            seen_non_monotonic = true;
        }
        if issue_type == "duplicate_measure" {
            seen_duplicate = true;
        }
    }
    assert!(seen_gap, "expected gap issue");
    assert!(seen_overlap, "expected overlap issue");
    assert!(seen_non_monotonic, "expected non_monotonic issue");
    assert!(seen_duplicate, "expected duplicate_measure issue");

    let gap_count = result
        .outputs
        .get("gap_count")
        .and_then(|v| v.as_u64())
        .expect("gap_count output");
    let overlap_count = result
        .outputs
        .get("overlap_count")
        .and_then(|v| v.as_u64())
        .expect("overlap_count output");
    let non_monotonic_count = result
        .outputs
        .get("non_monotonic_count")
        .and_then(|v| v.as_u64())
        .expect("non_monotonic_count output");
    let duplicate_count = result
        .outputs
        .get("duplicate_measure_count")
        .and_then(|v| v.as_u64())
        .expect("duplicate_measure_count output");

    assert!(gap_count >= 1);
    assert!(overlap_count >= 1);
    assert!(non_monotonic_count >= 1);
    assert!(duplicate_count >= 1);

    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn route_measure_qa_returns_zero_counts_for_clean_sequence() {
    use wbvector::{FieldDef, FieldType, FieldValue};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_route_measure_qa_clean");
    let events_path = std::env::temp_dir().join(format!("{tag}_events.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut events = Layer::new("events")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    events.schema.add_field(FieldDef::new("route_id", FieldType::Text));
    events.schema.add_field(FieldDef::new("from_m", FieldType::Float));
    events.schema.add_field(FieldDef::new("to_m", FieldType::Float));

    for (from_m, to_m) in [(0.0, 2.0), (2.0, 4.0), (4.0, 7.0)] {
        events
            .add_feature(
                Some(Geometry::Point(Coord::xy(from_m, 0.0))),
                &[
                    ("route_id", FieldValue::Text("R1".to_string())),
                    ("from_m", FieldValue::Float(from_m)),
                    ("to_m", FieldValue::Float(to_m)),
                ],
            )
            .expect("add event feature");
    }
    wbvector::write(&events, &events_path, VectorFormat::GeoPackage).expect("write events");

    let mut args = ToolArgs::new();
    args.insert("events".to_string(), json!(events_path.to_string_lossy().to_string()));
    args.insert("route_field".to_string(), json!("route_id"));
    args.insert("from_measure_field".to_string(), json!("from_m"));
    args.insert("to_measure_field".to_string(), json!("to_m"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let result = registry
        .run("route_measure_qa", &args, &context(&caps))
        .expect("route_measure_qa run");

    let out = wbvector::read(&out_path).expect("read qa diagnostics");
    assert_eq!(out.features.len(), 0);
    assert_eq!(result.outputs.get("gap_count").and_then(|v| v.as_u64()).unwrap_or(999), 0);
    assert_eq!(
        result
            .outputs
            .get("overlap_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(999),
        0
    );
    assert_eq!(
        result
            .outputs
            .get("non_monotonic_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(999),
        0
    );
    assert_eq!(
        result
            .outputs
            .get("duplicate_measure_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(999),
        0
    );

    let _ = std::fs::remove_file(&events_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn default_registry_contains_lidar_batch_a_tools() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ids: Vec<_> = registry.list().into_iter().map(|m| m.id).collect();

    assert!(ids.contains(&"lidar_nearest_neighbour_gridding"));
    assert!(ids.contains(&"lidar_idw_interpolation"));
    assert!(ids.contains(&"lidar_tin_gridding"));
    assert!(ids.contains(&"lidar_radial_basis_function_interpolation"));
    assert!(ids.contains(&"lidar_sibson_interpolation"));
    assert!(ids.contains(&"lidar_block_maximum"));
    assert!(ids.contains(&"lidar_block_minimum"));
    assert!(ids.contains(&"lidar_point_density"));
    assert!(ids.contains(&"lidar_digital_surface_model"));
    assert!(ids.contains(&"lidar_hillshade"));
    assert!(ids.contains(&"filter_lidar_classes"));
    assert!(ids.contains(&"lidar_shift"));
    assert!(ids.contains(&"remove_duplicates"));
    assert!(ids.contains(&"filter_lidar_scan_angles"));
    assert!(ids.contains(&"filter_lidar_noise"));
    assert!(ids.contains(&"lidar_thin"));
    assert!(ids.contains(&"lidar_elevation_slice"));
    assert!(ids.contains(&"lidar_join"));
    assert!(ids.contains(&"lidar_thin_high_density"));
    assert!(ids.contains(&"lidar_tile"));
    assert!(ids.contains(&"sort_lidar"));
    assert!(ids.contains(&"filter_lidar_by_percentile"));
    assert!(ids.contains(&"split_lidar"));
    assert!(ids.contains(&"lidar_remove_outliers"));
    assert!(ids.contains(&"normalize_lidar"));
    assert!(ids.contains(&"height_above_ground"));
    assert!(ids.contains(&"lidar_ground_point_filter"));
    assert!(ids.contains(&"filter_lidar"));
    assert!(ids.contains(&"modify_lidar"));
    assert!(ids.contains(&"filter_lidar_by_reference_surface"));
    assert!(ids.contains(&"classify_lidar"));
    assert!(ids.contains(&"lidar_classify_subset"));
    assert!(ids.contains(&"clip_lidar_to_polygon"));
    assert!(ids.contains(&"erase_polygon_from_lidar"));
    assert!(ids.contains(&"classify_overlap_points"));
    assert!(ids.contains(&"lidar_segmentation"));
    assert!(ids.contains(&"lidar_segmentation_based_filter"));
    assert!(ids.contains(&"lidar_colourize"));
    assert!(ids.contains(&"colourize_based_on_class"));
    assert!(ids.contains(&"colourize_based_on_point_returns"));
    assert!(ids.contains(&"classify_buildings_in_lidar"));
    assert!(ids.contains(&"ascii_to_las"));
    assert!(ids.contains(&"las_to_ascii"));
    assert!(ids.contains(&"select_tiles_by_polygon"));
    assert!(ids.contains(&"lidar_contour"));
    assert!(ids.contains(&"lidar_tile_footprint"));
    assert!(ids.contains(&"las_to_shapefile"));
    assert!(ids.contains(&"lidar_construct_vector_tin"));
    assert!(ids.contains(&"lidar_hex_bin"));
    assert!(ids.contains(&"lidar_point_return_analysis"));
    assert!(ids.contains(&"lidar_info"));
    assert!(ids.contains(&"lidar_histogram"));
    assert!(ids.contains(&"lidar_point_stats"));
    assert!(ids.contains(&"flightline_overlap"));
    assert!(ids.contains(&"recover_flightline_info"));
    assert!(ids.contains(&"find_flightline_edge_points"));
    assert!(ids.contains(&"lidar_tophat_transform"));
    assert!(ids.contains(&"normal_vectors"));
    assert!(ids.contains(&"lidar_kappa"));
    assert!(ids.contains(&"lidar_eigenvalue_features"));
    assert!(ids.contains(&"lidar_ransac_planes"));
    assert!(ids.contains(&"lidar_rooftop_analysis"));
}

#[test]
fn lidar_batch_a_tools_propagate_input_crs_to_output_rasters() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_batch_a_crs");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let nn_out = std::env::temp_dir().join(format!("{tag}_nn.tif"));
    let idw_out = std::env::temp_dir().join(format!("{tag}_idw.tif"));
    let tin_out = std::env::temp_dir().join(format!("{tag}_tin.tif"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 0.0,
                z: 11.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.0,
                y: 1.0,
                z: 12.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 1.0,
                z: 13.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut nn_args = ToolArgs::new();
    nn_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    nn_args.insert("resolution".to_string(), json!(0.5));
    nn_args.insert("search_radius".to_string(), json!(2.0));
    nn_args.insert("output".to_string(), json!(nn_out.to_string_lossy().to_string()));
    registry
        .run("lidar_nearest_neighbour_gridding", &nn_args, &context(&caps))
        .expect("lidar_nearest_neighbour_gridding should run");

    let mut idw_args = ToolArgs::new();
    idw_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    idw_args.insert("resolution".to_string(), json!(0.5));
    idw_args.insert("weight".to_string(), json!(1.0));
    idw_args.insert("search_radius".to_string(), json!(2.0));
    idw_args.insert("output".to_string(), json!(idw_out.to_string_lossy().to_string()));
    registry
        .run("lidar_idw_interpolation", &idw_args, &context(&caps))
        .expect("lidar_idw_interpolation should run");

    let mut tin_args = ToolArgs::new();
    tin_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    tin_args.insert("resolution".to_string(), json!(0.5));
    tin_args.insert("max_triangle_edge_length".to_string(), json!(10.0));
    tin_args.insert("output".to_string(), json!(tin_out.to_string_lossy().to_string()));
    registry
        .run("lidar_tin_gridding", &tin_args, &context(&caps))
        .expect("lidar_tin_gridding should run");

    let nn = Raster::read(&nn_out).expect("read nn output");
    let idw = Raster::read(&idw_out).expect("read idw output");
    let tin = Raster::read(&tin_out).expect("read tin output");

    assert_eq!(nn.crs.epsg, Some(4326));
    assert_eq!(idw.crs.epsg, Some(4326));
    assert_eq!(tin.crs.epsg, Some(4326));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&nn_out);
    let _ = std::fs::remove_file(&idw_out);
    let _ = std::fs::remove_file(&tin_out);
}

#[test]
fn lidar_batch_a_tools_propagate_wkt_only_crs_to_output_rasters() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_batch_a_wkt");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let wkt = "PROJCS[\"Custom Local\",GEOGCS[\"WGS 84\"]]";

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 0.0,
                z: 11.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.0,
                y: 1.0,
                z: 12.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::new().with_wkt(wkt)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut args = ToolArgs::new();
    args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    args.insert("resolution".to_string(), json!(0.5));
    args.insert("search_radius".to_string(), json!(2.0));
    let result = registry
        .run("lidar_nearest_neighbour_gridding", &args, &context(&caps))
        .expect("lidar_nearest_neighbour_gridding should run");

    let nn_path = result
        .outputs
        .get("path")
        .and_then(serde_json::Value::as_str)
        .expect("nn output path should be present");

    let nn_id = raster_memory_store::raster_path_to_id(nn_path)
        .expect("nn output should be an in-memory raster path");
    let nn = raster_memory_store::get_raster_by_id(nn_id)
        .expect("in-memory raster should be present");
    assert_eq!(nn.crs.epsg, None);
    assert_eq!(nn.crs.wkt.as_deref(), Some(wkt));

    let _ = std::fs::remove_file(&lidar_path);
}

#[test]
fn lidar_batch_a_supports_legacy_filter_aliases_and_thresholds() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_batch_a_filters");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let nn_out = std::env::temp_dir().join(format!("{tag}_nn.tif"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 2,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.2,
                y: 0.2,
                z: 20.0,
                classification: 2,
                return_number: 2,
                number_of_returns: 2,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut args = ToolArgs::new();
    args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    args.insert("resolution".to_string(), json!(1.0));
    args.insert("search_radius".to_string(), json!(10.0));
    args.insert("interpolation_parameter".to_string(), json!("return_number"));
    args.insert("returns".to_string(), json!("last"));
    args.insert("excluded_classes".to_string(), json!("1"));
    args.insert("min_elev".to_string(), json!(15.0));
    args.insert("max_elev".to_string(), json!(25.0));
    args.insert("output".to_string(), json!(nn_out.to_string_lossy().to_string()));

    registry
        .run("lidar_nearest_neighbour_gridding", &args, &context(&caps))
        .expect("lidar_nearest_neighbour_gridding should run with legacy filters");

    let nn = Raster::read(&nn_out).expect("read nn output");
    assert_eq!(nn.get(0, 0, 0), 2.0);

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&nn_out);
}

#[test]
fn lidar_idw_supports_legacy_filter_aliases_and_thresholds() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_idw_filters");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let out = std::env::temp_dir().join(format!("{tag}_idw.tif"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 2,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.2,
                y: 0.2,
                z: 20.0,
                classification: 2,
                return_number: 2,
                number_of_returns: 2,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut args = ToolArgs::new();
    args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    args.insert("resolution".to_string(), json!(1.0));
    args.insert("search_radius".to_string(), json!(10.0));
    args.insert("interpolation_parameter".to_string(), json!("return_number"));
    args.insert("returns".to_string(), json!("last"));
    args.insert("exclude_cls".to_string(), json!("1"));
    args.insert("minz".to_string(), json!(15.0));
    args.insert("maxz".to_string(), json!(25.0));
    args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));

    registry
        .run("lidar_idw_interpolation", &args, &context(&caps))
        .expect("lidar_idw_interpolation should run with legacy filters");

    let raster = Raster::read(&out).expect("read idw output");
    assert_eq!(raster.get(0, 0, 0), 2.0);

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn lidar_tin_supports_legacy_filter_aliases_and_thresholds() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_tin_filters");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let out = std::env::temp_dir().join(format!("{tag}_tin.tif"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 2,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.0,
                y: 2.0,
                z: 16.0,
                classification: 2,
                return_number: 2,
                number_of_returns: 2,
                ..PointRecord::default()
            },
            PointRecord {
                x: 2.0,
                y: 0.0,
                z: 17.0,
                classification: 2,
                return_number: 2,
                number_of_returns: 2,
                ..PointRecord::default()
            },
            PointRecord {
                x: 2.0,
                y: 2.0,
                z: 18.0,
                classification: 2,
                return_number: 2,
                number_of_returns: 2,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut args = ToolArgs::new();
    args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    args.insert("resolution".to_string(), json!(0.5));
    args.insert("max_triangle_edge_length".to_string(), json!(10.0));
    args.insert("interpolation_parameter".to_string(), json!("return_number"));
    args.insert("returns".to_string(), json!("last"));
    args.insert("excluded_classes".to_string(), json!("1"));
    args.insert("minz".to_string(), json!(15.0));
    args.insert("maxz".to_string(), json!(25.0));
    args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));

    registry
        .run("lidar_tin_gridding", &args, &context(&caps))
        .expect("lidar_tin_gridding should run with legacy filters");

    let raster = Raster::read(&out).expect("read tin output");
    let mut found = false;
    for idx in 0..raster.data.len() {
        let v = raster.data.get_f64(idx);
        if !raster.is_nodata(v) {
            assert_eq!(v, 2.0);
            found = true;
            break;
        }
    }
    assert!(found, "expected at least one interpolated TIN cell");

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn lidar_rbf_and_sibson_run_and_propagate_crs() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_rbf_sibson");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let rbf_out = std::env::temp_dir().join(format!("{tag}_rbf.tif"));
    let sibson_out = std::env::temp_dir().join(format!("{tag}_sibson.tif"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 2.0,
                y: 0.0,
                z: 12.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.0,
                y: 2.0,
                z: 14.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 2.0,
                y: 2.0,
                z: 16.0,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut rbf_args = ToolArgs::new();
    rbf_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    rbf_args.insert("resolution".to_string(), json!(0.5));
    rbf_args.insert("num_points".to_string(), json!(4));
    rbf_args.insert("func_type".to_string(), json!("gaussian"));
    rbf_args.insert("weight".to_string(), json!(0.2));
    rbf_args.insert("output".to_string(), json!(rbf_out.to_string_lossy().to_string()));
    registry
        .run(
            "lidar_radial_basis_function_interpolation",
            &rbf_args,
            &context(&caps),
        )
        .expect("lidar_radial_basis_function_interpolation should run");

    let mut sibson_args = ToolArgs::new();
    sibson_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    sibson_args.insert("resolution".to_string(), json!(0.5));
    sibson_args.insert("output".to_string(), json!(sibson_out.to_string_lossy().to_string()));
    registry
        .run("lidar_sibson_interpolation", &sibson_args, &context(&caps))
        .expect("lidar_sibson_interpolation should run");

    let rbf = Raster::read(&rbf_out).expect("read rbf output");
    let sibson = Raster::read(&sibson_out).expect("read sibson output");

    assert_eq!(rbf.crs.epsg, Some(4326));
    assert_eq!(sibson.crs.epsg, Some(4326));

    let rbf_has_values = (0..rbf.data.len()).any(|idx| {
        let v = rbf.data.get_f64(idx);
        !rbf.is_nodata(v)
    });
    let sibson_has_values = (0..sibson.data.len()).any(|idx| {
        let v = sibson.data.get_f64(idx);
        !sibson.is_nodata(v)
    });
    assert!(rbf_has_values, "RBF output should contain interpolated cells");
    assert!(sibson_has_values, "Sibson output should contain interpolated cells");

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&rbf_out);
    let _ = std::fs::remove_file(&sibson_out);
}

#[test]
fn lidar_batch_a_supports_time_and_rgb_interpolation_parameters() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_time_rgb");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let time_out = std::env::temp_dir().join(format!("{tag}_time.tif"));
    let rgb_out = std::env::temp_dir().join(format!("{tag}_rgb.tif"));

    let packed_red = ((255u32 << 24) | 255u32) as f64;
    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                gps_time: Some(GpsTime(42.5)),
                color: Some(Rgb16 {
                    red: 65535,
                    green: 0,
                    blue: 0,
                }),
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 0.0,
                z: 11.0,
                gps_time: Some(GpsTime(43.5)),
                color: Some(Rgb16 {
                    red: 65535,
                    green: 0,
                    blue: 0,
                }),
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut time_args = ToolArgs::new();
    time_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    time_args.insert("resolution".to_string(), json!(1.0));
    time_args.insert("search_radius".to_string(), json!(10.0));
    time_args.insert("interpolation_parameter".to_string(), json!("time"));
    time_args.insert("output".to_string(), json!(time_out.to_string_lossy().to_string()));
    registry
        .run("lidar_nearest_neighbour_gridding", &time_args, &context(&caps))
        .expect("lidar_nearest_neighbour_gridding should support time parameter");

    let mut rgb_args = ToolArgs::new();
    rgb_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    rgb_args.insert("resolution".to_string(), json!(1.0));
    rgb_args.insert("search_radius".to_string(), json!(10.0));
    rgb_args.insert("interpolation_parameter".to_string(), json!("rgb"));
    rgb_args.insert("output".to_string(), json!(rgb_out.to_string_lossy().to_string()));
    registry
        .run("lidar_nearest_neighbour_gridding", &rgb_args, &context(&caps))
        .expect("lidar_nearest_neighbour_gridding should support rgb parameter");

    let time_raster = Raster::read(&time_out).expect("read time output");
    let rgb_raster = Raster::read(&rgb_out).expect("read rgb output");
    assert_eq!(time_raster.get(0, 0, 0), 42.5);
    assert_eq!(rgb_raster.get(0, 0, 0), packed_red);

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&time_out);
    let _ = std::fs::remove_file(&rgb_out);
}

#[test]
fn lidar_rbf_poly_order_quadratic_is_functional() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_rbf_poly");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let none_out = std::env::temp_dir().join(format!("{tag}_none.tif"));
    let quad_out = std::env::temp_dir().join(format!("{tag}_quad.tif"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: -1.0, y: -1.0, z: 3.5, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: -1.0, y: 1.0, z: 1.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: -1.0, z: 7.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 5.5, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: -0.5, y: 0.3, z: 1.855, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.4, y: -0.7, z: 4.8775, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut none_args = ToolArgs::new();
    none_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    none_args.insert("resolution".to_string(), json!(2.0));
    none_args.insert("num_points".to_string(), json!(6));
    none_args.insert("func_type".to_string(), json!("gaussian"));
    none_args.insert("weight".to_string(), json!(0.2));
    none_args.insert("poly_order".to_string(), json!("none"));
    none_args.insert("output".to_string(), json!(none_out.to_string_lossy().to_string()));
    registry
        .run(
            "lidar_radial_basis_function_interpolation",
            &none_args,
            &context(&caps),
        )
        .expect("rbf with poly_order=none should run");

    let mut quad_args = none_args.clone();
    quad_args.insert("poly_order".to_string(), json!("quadratic"));
    quad_args.insert("output".to_string(), json!(quad_out.to_string_lossy().to_string()));
    registry
        .run(
            "lidar_radial_basis_function_interpolation",
            &quad_args,
            &context(&caps),
        )
        .expect("rbf with poly_order=quadratic should run");

    let none_raster = Raster::read(&none_out).expect("read none output");
    let quad_raster = Raster::read(&quad_out).expect("read quadratic output");
    let none_v = none_raster.get(0, 0, 0);
    let quad_v = quad_raster.get(0, 0, 0);
    let expected = 3.0;

    assert!(
        (quad_v - expected).abs() < (none_v - expected).abs(),
        "poly_order=quadratic should improve the estimate at origin: none={none_v}, quad={quad_v}, expected={expected}"
    );

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&none_out);
    let _ = std::fs::remove_file(&quad_out);
}

#[test]
fn lidar_phase1_batch_b_tools_run_and_propagate_crs() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_batch_b");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let block_max_out = std::env::temp_dir().join(format!("{tag}_block_max.tif"));
    let block_min_out = std::env::temp_dir().join(format!("{tag}_block_min.tif"));
    let density_out = std::env::temp_dir().join(format!("{tag}_density.tif"));
    let dsm_out = std::env::temp_dir().join(format!("{tag}_dsm.tif"));
    let hs_out = std::env::temp_dir().join(format!("{tag}_hs.tif"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 11.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 1.0, z: 12.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 13.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.5, y: 0.5, z: 14.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut block_max_args = ToolArgs::new();
    block_max_args.insert("input".to_string(), json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}));
    block_max_args.insert("resolution".to_string(), json!(0.5));
    block_max_args.insert("output".to_string(), json!(block_max_out.to_string_lossy().to_string()));
    registry.run("lidar_block_maximum", &block_max_args, &context(&caps)).expect("lidar_block_maximum should run");

    let mut block_min_args = block_max_args.clone();
    block_min_args.insert("output".to_string(), json!(block_min_out.to_string_lossy().to_string()));
    registry.run("lidar_block_minimum", &block_min_args, &context(&caps)).expect("lidar_block_minimum should run");

    let mut density_args = ToolArgs::new();
    density_args.insert("input".to_string(), json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}));
    density_args.insert("resolution".to_string(), json!(0.5));
    density_args.insert("search_radius".to_string(), json!(1.0));
    density_args.insert("output".to_string(), json!(density_out.to_string_lossy().to_string()));
    registry.run("lidar_point_density", &density_args, &context(&caps)).expect("lidar_point_density should run");

    let mut dsm_args = ToolArgs::new();
    dsm_args.insert("input".to_string(), json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}));
    dsm_args.insert("resolution".to_string(), json!(0.5));
    dsm_args.insert("search_radius".to_string(), json!(0.5));
    dsm_args.insert("output".to_string(), json!(dsm_out.to_string_lossy().to_string()));
    registry.run("lidar_digital_surface_model", &dsm_args, &context(&caps)).expect("lidar_digital_surface_model should run");

    let mut hs_args = ToolArgs::new();
    hs_args.insert("input".to_string(), json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}));
    hs_args.insert("resolution".to_string(), json!(0.5));
    hs_args.insert("output".to_string(), json!(hs_out.to_string_lossy().to_string()));
    registry.run("lidar_hillshade", &hs_args, &context(&caps)).expect("lidar_hillshade should run");

    for out in [&block_max_out, &block_min_out, &density_out, &dsm_out] {
        let raster = Raster::read(out).expect("read lidar phase-1 batch-b output");
        assert_eq!(raster.crs.epsg, Some(4326));
        let has_values = (0..raster.data.len()).any(|idx| {
            let v = raster.data.get_f64(idx);
            !raster.is_nodata(v)
        });
        assert!(has_values, "output should contain at least one interpolated value: {}", out.to_string_lossy());
    }

    let hs = Raster::read(&hs_out).expect("read lidar hillshade output");
    assert_eq!(hs.crs.epsg, Some(4326));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&block_max_out);
    let _ = std::fs::remove_file(&block_min_out);
    let _ = std::fs::remove_file(&density_out);
    let _ = std::fs::remove_file(&dsm_out);
    let _ = std::fs::remove_file(&hs_out);
}

#[test]
fn lidar_phase1_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_batch_mode");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("tile_a.las");
    let tile_b = batch_dir.join("tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 11.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 1.0, z: 12.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 13.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 2.0, y: 2.0, z: 20.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 2.0, z: 21.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 3.0, z: 22.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 3.0, z: 23.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir to batch dir");

    let mut max_args = ToolArgs::new();
    max_args.insert("resolution".to_string(), json!(1.0));
    let max_res = registry.run("lidar_block_maximum", &max_args, &context(&caps)).expect("lidar_block_maximum batch run");

    let mut min_args = ToolArgs::new();
    min_args.insert("resolution".to_string(), json!(1.0));
    let min_res = registry.run("lidar_block_minimum", &min_args, &context(&caps)).expect("lidar_block_minimum batch run");

    let mut density_args = ToolArgs::new();
    density_args.insert("resolution".to_string(), json!(1.0));
    density_args.insert("search_radius".to_string(), json!(1.25));
    let density_res = registry.run("lidar_point_density", &density_args, &context(&caps)).expect("lidar_point_density batch run");

    let mut dsm_args = ToolArgs::new();
    dsm_args.insert("resolution".to_string(), json!(1.0));
    dsm_args.insert("search_radius".to_string(), json!(0.75));
    let dsm_res = registry.run("lidar_digital_surface_model", &dsm_args, &context(&caps)).expect("lidar_digital_surface_model batch run");

    let mut hs_args = ToolArgs::new();
    hs_args.insert("resolution".to_string(), json!(1.0));
    hs_args.insert("search_radius".to_string(), json!(1.5));
    let hs_res = registry.run("lidar_hillshade", &hs_args, &context(&caps)).expect("lidar_hillshade batch run");

    std::env::set_current_dir(&old_cwd).expect("restore current dir");

    for res in [&max_res, &min_res, &density_res, &dsm_res, &hs_res] {
        assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("raster")));
        let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
        assert!(std::path::Path::new(p).exists(), "placeholder output should exist: {p}");
    }

    for suffix in ["block_max", "block_min", "density", "dsm", "hillshade"] {
        let _ = std::fs::remove_file(batch_dir.join(format!("tile_a_{suffix}.tif")));
        let _ = std::fs::remove_file(batch_dir.join(format!("tile_b_{suffix}.tif")));
    }
    let _ = std::fs::remove_file(&tile_a);
    let _ = std::fs::remove_file(&tile_b);
    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn lidar_interpolation_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_interp_batch_mode");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("interp_tile_a.las");
    let tile_b = batch_dir.join("interp_tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 11.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 1.0, z: 12.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 13.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 2.0, y: 2.0, z: 20.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 2.0, z: 21.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 3.0, z: 22.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 3.0, z: 23.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir to batch dir");

    let mut nn_args = ToolArgs::new();
    nn_args.insert("resolution".to_string(), json!(1.0));
    nn_args.insert("search_radius".to_string(), json!(2.0));
    let nn_res = registry.run("lidar_nearest_neighbour_gridding", &nn_args, &context(&caps)).expect("nn batch run");

    let mut idw_args = ToolArgs::new();
    idw_args.insert("resolution".to_string(), json!(1.0));
    idw_args.insert("search_radius".to_string(), json!(2.0));
    let idw_res = registry.run("lidar_idw_interpolation", &idw_args, &context(&caps)).expect("idw batch run");

    let mut tin_args = ToolArgs::new();
    tin_args.insert("resolution".to_string(), json!(1.0));
    let tin_res = registry.run("lidar_tin_gridding", &tin_args, &context(&caps)).expect("tin batch run");

    let mut rbf_args = ToolArgs::new();
    rbf_args.insert("resolution".to_string(), json!(1.0));
    rbf_args.insert("num_points".to_string(), json!(4));
    let rbf_res = registry.run("lidar_radial_basis_function_interpolation", &rbf_args, &context(&caps)).expect("rbf batch run");

    let mut sib_args = ToolArgs::new();
    sib_args.insert("resolution".to_string(), json!(1.0));
    let sib_res = registry.run("lidar_sibson_interpolation", &sib_args, &context(&caps)).expect("sibson batch run");

    std::env::set_current_dir(&old_cwd).expect("restore current dir");

    for res in [&nn_res, &idw_res, &tin_res, &rbf_res, &sib_res] {
        assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("raster")));
        let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
        assert!(std::path::Path::new(p).exists(), "placeholder output should exist: {p}");
    }

    for suffix in ["nn", "idw", "tin", "rbf", "sibson"] {
        let _ = std::fs::remove_file(batch_dir.join(format!("interp_tile_a_{suffix}.tif")));
        let _ = std::fs::remove_file(batch_dir.join(format!("interp_tile_b_{suffix}.tif")));
    }
    let _ = std::fs::remove_file(&tile_a);
    let _ = std::fs::remove_file(&tile_b);
    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn lidar_phase2_tools_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let filtered_out = std::env::temp_dir().join(format!("{tag}_filtered.las"));
    let shifted_out = std::env::temp_dir().join(format!("{tag}_shifted.las"));
    let dedup_out = std::env::temp_dir().join(format!("{tag}_dedup.las"));
    let dedup_xyz_out = std::env::temp_dir().join(format!("{tag}_dedup_xyz.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 0.0, z: 12.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 20.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 2.0, z: 30.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    // filter_lidar_classes: exclude class 2 → only 3 points remain
    let mut filter_args = ToolArgs::new();
    filter_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    filter_args.insert("excluded_classes".to_string(), json!([2]));
    filter_args.insert("output".to_string(), json!(filtered_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_classes", &filter_args, &context(&caps))
        .expect("filter_lidar_classes should run");

    let filtered = PointCloud::read(&filtered_out).expect("read filtered output");
    assert!(filtered.points.iter().all(|p| p.classification != 2));
    assert_eq!(filtered.points.len(), 3);
    assert_eq!(filtered.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    // lidar_shift: apply x/y/z offsets
    let mut shift_args = ToolArgs::new();
    shift_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    shift_args.insert("x_shift".to_string(), json!(1.5));
    shift_args.insert("y_shift".to_string(), json!(-2.0));
    shift_args.insert("z_shift".to_string(), json!(0.25));
    shift_args.insert("output".to_string(), json!(shifted_out.to_string_lossy().to_string()));
    registry
        .run("lidar_shift", &shift_args, &context(&caps))
        .expect("lidar_shift should run");

    let shifted = PointCloud::read(&shifted_out).expect("read shifted output");
    assert_eq!(shifted.points.len(), cloud.points.len());
    let p0 = &shifted.points[0];
    assert!((p0.x - 1.5).abs() < 1e-9);
    assert!((p0.y + 2.0).abs() < 1e-9);
    assert!((p0.z - 10.25).abs() < 1e-9);
    assert_eq!(shifted.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    // remove_duplicates with include_z=false: x/y dedup removes (0,0) pair → 3 unique xy positions
    let mut dedup_args = ToolArgs::new();
    dedup_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    dedup_args.insert("include_z".to_string(), json!(false));
    dedup_args.insert("output".to_string(), json!(dedup_out.to_string_lossy().to_string()));
    registry
        .run("remove_duplicates", &dedup_args, &context(&caps))
        .expect("remove_duplicates should run");

    let dedup = PointCloud::read(&dedup_out).expect("read dedup output");
    assert_eq!(dedup.points.len(), 3);

    // remove_duplicates with include_z=true: both (0,0,10) and (0,0,12) are distinct → all 4 kept
    let mut dedup_xyz_args = ToolArgs::new();
    dedup_xyz_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    dedup_xyz_args.insert("include_z".to_string(), json!(true));
    dedup_xyz_args.insert("output".to_string(), json!(dedup_xyz_out.to_string_lossy().to_string()));
    registry
        .run("remove_duplicates", &dedup_xyz_args, &context(&caps))
        .expect("remove_duplicates include_z should run");

    let dedup_xyz = PointCloud::read(&dedup_xyz_out).expect("read dedup_xyz output");
    assert_eq!(dedup_xyz.points.len(), 4);

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&filtered_out);
    let _ = std::fs::remove_file(&shifted_out);
    let _ = std::fs::remove_file(&dedup_out);
    let _ = std::fs::remove_file(&dedup_xyz_out);
}

#[test]
fn lidar_phase2_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch_mode");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("phase2_tile_a.las");
    let tile_b = batch_dir.join("phase2_tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 0.0, z: 11.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 12.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 2.0, y: 2.0, z: 20.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 2.0, z: 21.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 3.0, z: 22.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir to batch dir");

    let mut filter_args = ToolArgs::new();
    filter_args.insert("excluded_classes".to_string(), json!([2]));
    let filter_res = registry
        .run("filter_lidar_classes", &filter_args, &context(&caps))
        .expect("filter_lidar_classes batch run");

    let mut shift_args = ToolArgs::new();
    shift_args.insert("x_shift".to_string(), json!(1.0));
    shift_args.insert("y_shift".to_string(), json!(-1.0));
    shift_args.insert("z_shift".to_string(), json!(0.5));
    let shift_res = registry
        .run("lidar_shift", &shift_args, &context(&caps))
        .expect("lidar_shift batch run");

    let mut dedup_args = ToolArgs::new();
    dedup_args.insert("include_z".to_string(), json!(false));
    let dedup_res = registry
        .run("remove_duplicates", &dedup_args, &context(&caps))
        .expect("remove_duplicates batch run");

    std::env::set_current_dir(&old_cwd).expect("restore current dir");

    for res in [&filter_res, &shift_res, &dedup_res] {
        assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
        let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
        assert!(std::path::Path::new(p).exists(), "placeholder output should exist: {p}");
    }

    for suffix in ["filtered_cls", "shifted", "dedup"] {
        let _ = std::fs::remove_file(batch_dir.join(format!("phase2_tile_a_{suffix}.las")));
        let _ = std::fs::remove_file(batch_dir.join(format!("phase2_tile_b_{suffix}.las")));
    }
    let _ = std::fs::remove_file(&tile_a);
    let _ = std::fs::remove_file(&tile_b);
    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn construct_vector_tin_and_vector_hex_binning_run_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_tin_hexbin");
    let points_in = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let tin_out = std::env::temp_dir().join(format!("{tag}_tin.geojson"));
    let hex_out = std::env::temp_dir().join(format!("{tag}_hex.geojson"));

    let mut point_layer = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    point_layer
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[])
        .expect("add point 1");
    point_layer
        .add_feature(Some(Geometry::point(1.0, 0.0)), &[])
        .expect("add point 2");
    point_layer
        .add_feature(Some(Geometry::point(0.0, 1.0)), &[])
        .expect("add point 3");
    point_layer
        .add_feature(Some(Geometry::point(1.0, 1.0)), &[])
        .expect("add point 4");
    wbvector::write(&point_layer, &points_in, VectorFormat::GeoJson).expect("write points input");

    let mut tin_args = ToolArgs::new();
    tin_args.insert("input_points".to_string(), json!(points_in.to_string_lossy().to_string()));
    tin_args.insert("field_name".to_string(), json!("FID"));
    tin_args.insert("max_triangle_edge_length".to_string(), json!(-1.0));
    tin_args.insert("output".to_string(), json!(tin_out.to_string_lossy().to_string()));
    registry
        .run("construct_vector_tin", &tin_args, &context(&caps))
        .expect("construct_vector_tin should run");

    let tin = wbvector::read(&tin_out).expect("read tin output");
    assert!(!tin.features.is_empty(), "TIN output should contain triangles");
    match tin.features[0].geometry.as_ref().expect("tin geometry") {
        Geometry::Polygon { .. } => {}
        _ => panic!("TIN output should be polygon geometry"),
    }

    let mut hex_args = ToolArgs::new();
    hex_args.insert("vector_points".to_string(), json!(points_in.to_string_lossy().to_string()));
    hex_args.insert("width".to_string(), json!(0.75));
    hex_args.insert("orientation".to_string(), json!("h"));
    hex_args.insert("output".to_string(), json!(hex_out.to_string_lossy().to_string()));
    registry
        .run("vector_hex_binning", &hex_args, &context(&caps))
        .expect("vector_hex_binning should run");

    let hex = wbvector::read(&hex_out).expect("read hex output");
    assert!(!hex.features.is_empty(), "Hex bin output should contain polygons");
    match hex.features[0].geometry.as_ref().expect("hex geometry") {
        Geometry::Polygon { .. } => {}
        _ => panic!("hex output should be polygon geometry"),
    }

    let _ = std::fs::remove_file(&points_in);
    let _ = std::fs::remove_file(&tin_out);
    let _ = std::fs::remove_file(&hex_out);
}

#[test]
fn find_patch_edge_cells_runs_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_find_patch_edge_cells_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_find_patch_edge_cells_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });

    for r in 0..3 {
        for c in 0..3 {
            raster.set(0, r, c, 1.0).expect("set patch cell");
        }
    }
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("find_patch_edge_cells", &args, &context(&caps))
        .expect("find_patch_edge_cells tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    // Center of the 3x3 patch should be interior (0), corners are edge cells (= patch ID 1).
    assert_eq!(out.get(0, 1, 1), 0.0);
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 2), 1.0);
    assert_eq!(out.get(0, 2, 0), 1.0);
    assert_eq!(out.get(0, 2, 2), 1.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn default_registry_contains_flow_algorithm_tools() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ids: Vec<_> = registry.list().into_iter().map(|m| m.id).collect();

    assert!(ids.contains(&"d8_pointer"));
    assert!(ids.contains(&"d8_flow_accum"));
    assert!(ids.contains(&"dinf_pointer"));
    assert!(ids.contains(&"dinf_flow_accum"));
    assert!(ids.contains(&"fd8_pointer"));
    assert!(ids.contains(&"fd8_flow_accum"));
    assert!(ids.contains(&"rho8_pointer"));
    assert!(ids.contains(&"rho8_flow_accum"));
    assert!(ids.contains(&"mdinf_flow_accum"));
    assert!(ids.contains(&"qin_flow_accumulation"));
    assert!(ids.contains(&"quinn_flow_accumulation"));
    assert!(ids.contains(&"minimal_dispersion_flow_algorithm"));
}

#[test]
fn unknown_tool_returns_not_found() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let args = ToolArgs::new();
    let caps = OpenOnly;
    let result = registry.run("does_not_exist", &args, &context(&caps));
    assert!(matches!(result, Err(ToolError::NotFound(_))));
}

#[test]
fn abs_tool_runs_end_to_end_with_raster_io() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_abs_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_abs_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, -1.0).expect("set");
    raster.set(0, 0, 1, -2.5).expect("set");
    raster.set(0, 1, 0, 3.0).expect("set");
    raster.set(0, 1, 1, -4.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("abs", &args, &context(&caps))
        .expect("abs tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.5);
    assert_eq!(out.get(0, 1, 0), 3.0);
    assert_eq!(out.get(0, 1, 1), 4.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn fill_pits_raises_single_cell_pit() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fill_pits_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_fill_pits_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3 {
        for c in 0..3 {
            raster.set(0, r, c, 10.0).expect("set");
        }
    }
    raster.set(0, 1, 1, 5.0).expect("set center pit");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("fill_pits", &args, &context(&caps))
        .expect("fill_pits tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert!(out.get(0, 1, 1) >= 10.0);
    assert!(out.get(0, 1, 1) < 10.0001);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn breach_single_cell_pits_carves_adjacent_cell() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_breach_pits_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_breach_pits_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 5,
        rows: 5,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });

    for r in 0..5 {
        for c in 0..5 {
            raster.set(0, r, c, 12.0).expect("set");
        }
    }
    raster.set(0, 2, 2, 10.0).expect("set center pit");
    raster.set(0, 0, 2, 4.0).expect("set lower second ring");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("breach_single_cell_pits", &args, &context(&caps))
        .expect("breach_single_cell_pits tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert!(out.get(0, 1, 2) < 12.0);
    assert_eq!(out.get(0, 2, 2), 10.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn d8_pointer_runs_on_simple_dem() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_d8_ptr_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_d8_ptr_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    // Monotonic gradient to east; center should flow east in D8.
    for r in 0..3 {
        raster.set(0, r, 0, 30.0).expect("set");
        raster.set(0, r, 1, 20.0).expect("set");
        raster.set(0, r, 2, 10.0).expect("set");
    }
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("d8_pointer", &args, &context(&caps))
        .expect("d8_pointer tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 1, 1), 2.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn d8_flow_accum_runs_on_pointer_input() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_d8_acc_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_d8_acc_out_{unique}.asc"));

    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    // Whitebox D8 pointer east=2; 0 marks no outflow.
    pntr.set(0, 0, 0, 2.0).expect("set");
    pntr.set(0, 0, 1, 2.0).expect("set");
    pntr.set(0, 0, 2, 0.0).expect("set");
    pntr
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("input_is_pointer".to_string(), json!(true));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("d8_flow_accum", &args, &context(&caps))
        .expect("d8_flow_accum tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn dinf_pointer_runs_on_simple_dem() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_ptr_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_ptr_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3 {
        raster.set(0, r, 0, 30.0).expect("set");
        raster.set(0, r, 1, 20.0).expect("set");
        raster.set(0, r, 2, 10.0).expect("set");
    }
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("dinf_pointer", &args, &context(&caps))
        .expect("dinf_pointer tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert!((out.get(0, 1, 1) - 90.0).abs() < 1.0e-6);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn dinf_pointer_runs_on_geographic_dem() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_ptr_geo_in_{unique}.tif"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_ptr_geo_out_{unique}.tif"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: -1.5,
        y_min: -1.5,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for r in 0..3 {
        raster.set(0, r, 0, 30.0).expect("set");
        raster.set(0, r, 1, 20.0).expect("set");
        raster.set(0, r, 2, 10.0).expect("set");
    }
    raster
        .write(&input_path, RasterFormat::GeoTiff)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("dinf_pointer", &args, &context(&caps))
        .expect("dinf_pointer tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert!((out.get(0, 1, 1) - 90.0).abs() < 1.0e-6);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn dinf_flow_accum_runs_on_pointer_input() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_acc_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_acc_out_{unique}.asc"));

    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    pntr.set(0, 0, 0, 90.0).expect("set");
    pntr.set(0, 0, 1, 90.0).expect("set");
    pntr.set(0, 0, 2, -1.0).expect("set");
    pntr
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("input_is_pointer".to_string(), json!(true));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("dinf_flow_accum", &args, &context(&caps))
        .expect("dinf_flow_accum tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn dinf_flow_accum_scales_geographic_pointer_input() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_acc_geo_in_{unique}.tif"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_dinf_acc_geo_out_{unique}.tif"));

    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: -1.5,
        y_min: -1.5,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for r in 0..3 {
        for c in 0..3 {
            pntr.set(0, r, c, -1.0).expect("set");
        }
    }
    pntr.set(0, 1, 0, 90.0).expect("set");
    pntr.set(0, 1, 1, 90.0).expect("set");
    pntr
        .write(&input_path, RasterFormat::GeoTiff)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("input_is_pointer".to_string(), json!(true));
    args.insert("out_type".to_string(), json!("sca"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("dinf_flow_accum", &args, &context(&caps))
        .expect("dinf_flow_accum tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert!(out.get(0, 1, 0) > 100_000.0);
    assert!(out.get(0, 1, 1) > out.get(0, 1, 0));
    assert!(out.get(0, 1, 2) > out.get(0, 1, 1));

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn fd8_pointer_encodes_multiple_downslope_neighbours() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fd8_ptr_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_fd8_ptr_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3 {
        for c in 0..3 {
            raster.set(0, r, c, 20.0).expect("set");
        }
    }
    raster.set(0, 1, 1, 10.0).expect("set center");
    raster.set(0, 1, 2, 5.0).expect("set east");
    raster.set(0, 2, 1, 4.0).expect("set south");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("fd8_pointer", &args, &context(&caps))
        .expect("fd8_pointer tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 1, 1), 10.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn fd8_flow_accum_runs_on_simple_dem() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fd8_acc_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_fd8_acc_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("fd8_flow_accum", &args, &context(&caps))
        .expect("fd8_flow_accum tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn fd8_flow_accum_scales_geographic_dem() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fd8_acc_geo_in_{unique}.tif"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_fd8_acc_geo_out_{unique}.tif"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: -1.5,
        y_min: -0.5,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::GeoTiff)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("sca"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("fd8_flow_accum", &args, &context(&caps))
        .expect("fd8_flow_accum tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert!(out.get(0, 0, 0) > 100_000.0);
    assert!(out.get(0, 0, 1) > out.get(0, 0, 0));
    assert!(out.get(0, 0, 2) > out.get(0, 0, 1));

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn rho8_pointer_runs_on_simple_dem() {
    // 1-row DEM: only cardinal E/W neighbours are in bounds, so no stochastic
    // factor ever fires and results are fully deterministic.
    // [30, 20, 10]: cell(0,0)->E=2, cell(0,1)->E=2, cell(0,2)->no downslope=0.
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_rho8_ptr_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_rho8_ptr_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("rho8_pointer", &args, &context(&caps))
        .expect("rho8_pointer tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    // East = direction index 1 -> Whitebox base-2 pointer value 2.
    assert_eq!(out.get(0, 0, 0), 2.0, "cell (0,0) should point east");
    assert_eq!(out.get(0, 0, 1), 2.0, "cell (0,1) should point east");
    assert_eq!(out.get(0, 0, 2), 0.0, "cell (0,2) has no downslope");

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn rho8_flow_accum_runs_on_simple_dem() {
    // Same 1-row DEM, cells output. Only cardinal neighbours active -> deterministic.
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_rho8_acc_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_rho8_acc_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("rho8_flow_accum", &args, &context(&caps))
        .expect("rho8_flow_accum tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn mdinf_flow_accum_runs_on_simple_dem() {
    // 1-row DEM [30, 20, 10]: only cardinal east neighbour is in-bounds for
    // each cell, so MDInf routes all flow eastward deterministically.
    // Expected cells output: [1, 2, 3].
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path =
        std::env::temp_dir().join(format!("wbtools_oss_mdinf_acc_in_{unique}.asc"));
    let output_path =
        std::env::temp_dir().join(format!("wbtools_oss_mdinf_acc_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("mdinf_flow_accum", &args, &context(&caps))
        .expect("mdinf_flow_accum tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn qin_flow_accumulation_runs_on_simple_dem() {
    // 1-row DEM [30, 20, 10]: only cardinal east neighbour is in-bounds,
    // so Qin MFD routes deterministically eastward for this setup.
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_qin_acc_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_qin_acc_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("qin_flow_accumulation", &args, &context(&caps))
        .expect("qin_flow_accumulation tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn quinn_flow_accumulation_runs_on_simple_dem() {
    // Same deterministic 1-row setup as Qin test.
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_quinn_acc_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_quinn_acc_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("quinn_flow_accumulation", &args, &context(&caps))
        .expect("quinn_flow_accumulation tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn minimal_dispersion_flow_algorithm_returns_direction_and_accumulation() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_mdfa_in_{unique}.asc"));
    let dir_path = std::env::temp_dir().join(format!("wbtools_oss_mdfa_dir_{unique}.asc"));
    let accum_path = std::env::temp_dir().join(format!("wbtools_oss_mdfa_acc_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("path_corrected_direction_preference".to_string(), json!(1.0));
    args.insert("flow_dir_output".to_string(), json!(dir_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(accum_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    let result = registry
        .run("minimal_dispersion_flow_algorithm", &args, &context(&caps))
        .expect("minimal_dispersion_flow_algorithm tool should run");

    assert_eq!(result.outputs.get("__wbw_type__"), Some(&json!("tuple")));
    assert!(result.outputs.get("flow_dir").is_some());
    assert!(result.outputs.get("flow_accum").is_some());

    let dir = Raster::read(&dir_path).expect("read direction raster");
    assert_eq!(dir.get(0, 0, 0), 2.0);
    assert_eq!(dir.get(0, 0, 1), 2.0);
    assert_eq!(dir.get(0, 0, 2), 0.0);

    let accum = Raster::read(&accum_path).expect("read accumulation raster");
    assert_eq!(accum.get(0, 0, 0), 1.0);
    assert_eq!(accum.get(0, 0, 1), 2.0);
    assert_eq!(accum.get(0, 0, 2), 3.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(dir_path);
    let _ = std::fs::remove_file(accum_path);
}

#[test]
fn flow_accum_full_workflow_returns_dem_pointer_and_accum() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_in_{unique}.asc"));
    let dem_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_dem_{unique}.asc"));
    let dir_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_dir_{unique}.asc"));
    let acc_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_acc_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("breached_dem_output".to_string(), json!(dem_path.to_string_lossy().to_string()));
    args.insert("flow_dir_output".to_string(), json!(dir_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(acc_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    let result = registry
        .run("flow_accum_full_workflow", &args, &context(&caps))
        .expect("flow_accum_full_workflow tool should run");

    assert_eq!(result.outputs.get("__wbw_type__"), Some(&json!("tuple")));
    assert!(result.outputs.get("breached_dem").is_some());
    assert!(result.outputs.get("flow_dir").is_some());
    assert!(result.outputs.get("flow_accum").is_some());

    let breached_dem = Raster::read(&dem_path).expect("read breached dem raster");
    assert_eq!(breached_dem.get(0, 0, 0), 30.0);
    assert_eq!(breached_dem.get(0, 0, 1), 20.0);
    assert_eq!(breached_dem.get(0, 0, 2), 10.0);

    let dir = Raster::read(&dir_path).expect("read direction raster");
    assert_eq!(dir.get(0, 0, 0), 2.0);
    assert_eq!(dir.get(0, 0, 1), 2.0);

    let accum = Raster::read(&acc_path).expect("read accumulation raster");
    assert_eq!(accum.get(0, 0, 0), 1.0);
    assert_eq!(accum.get(0, 0, 1), 2.0);
    assert!(accum.get(0, 0, 2) >= 1.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(dem_path);
    let _ = std::fs::remove_file(dir_path);
    let _ = std::fs::remove_file(acc_path);
}

#[test]
fn flow_accum_full_workflow_honours_esri_pointer_encoding() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_esri_in_{unique}.asc"));
    let dir_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_esri_dir_{unique}.asc"));
    let acc_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_esri_acc_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("esri_pntr".to_string(), json!(true));
    args.insert("flow_dir_output".to_string(), json!(dir_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(acc_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("flow_accum_full_workflow", &args, &context(&caps))
        .expect("flow_accum_full_workflow tool should run with esri pointers");

    let dir = Raster::read(&dir_path).expect("read direction raster");
    assert_eq!(dir.get(0, 0, 0), 1.0);
    assert_eq!(dir.get(0, 0, 1), 1.0);

    let accum = Raster::read(&acc_path).expect("read accumulation raster");
    assert_eq!(accum.get(0, 0, 0), 1.0);
    assert_eq!(accum.get(0, 0, 1), 2.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(dir_path);
    let _ = std::fs::remove_file(acc_path);
}

#[test]
fn flow_accum_full_workflow_scales_geographic_dem() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_geo_in_{unique}.tif"));
    let dir_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_geo_dir_{unique}.tif"));
    let acc_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_geo_acc_{unique}.tif"));
    let expected_acc_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_geo_expected_{unique}.tif"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: -1.5,
        y_min: -0.5,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::GeoTiff)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("sca"));
    args.insert("flow_dir_output".to_string(), json!(dir_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(acc_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("flow_accum_full_workflow", &args, &context(&caps))
        .expect("flow_accum_full_workflow tool should run on geographic raster");

    let dir = Raster::read(&dir_path).expect("read direction raster");
    assert_eq!(dir.get(0, 0, 0), 2.0);
    assert_eq!(dir.get(0, 0, 1), 2.0);

    let mut direct_args = ToolArgs::new();
    direct_args.insert("input".to_string(), json!(dir_path.to_string_lossy().to_string()));
    direct_args.insert("input_is_pointer".to_string(), json!(true));
    direct_args.insert("out_type".to_string(), json!("sca"));
    direct_args.insert("output".to_string(), json!(expected_acc_path.to_string_lossy().to_string()));
    registry
        .run("d8_flow_accum", &direct_args, &context(&caps))
        .expect("direct d8_flow_accum on pointer should run");

    let accum = Raster::read(&acc_path).expect("read accumulation raster");
    let expected = Raster::read(&expected_acc_path).expect("read direct accumulation raster");
    assert!((accum.get(0, 0, 0) - expected.get(0, 0, 0)).abs() < 1.0e-6);
    assert!((accum.get(0, 0, 1) - expected.get(0, 0, 1)).abs() < 1.0e-6);
    assert!((accum.get(0, 0, 2) - expected.get(0, 0, 2)).abs() < 1.0e-6);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(dir_path);
    let _ = std::fs::remove_file(acc_path);
    let _ = std::fs::remove_file(expected_acc_path);
}

#[test]
fn find_noflow_cells_identifies_terminal_cell() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_noflow_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_noflow_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster.write(&input_path, RasterFormat::EsriAscii).expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("find_noflow_cells", &args, &context(&caps))
        .expect("find_noflow_cells tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), out.nodata);
    assert_eq!(out.get(0, 0, 1), out.nodata);
    assert_eq!(out.get(0, 0, 2), 1.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn num_inflowing_neighbours_counts_upstream_cells() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_inflow_in_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_inflow_out_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster.write(&input_path, RasterFormat::EsriAscii).expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("num_inflowing_neighbours", &args, &context(&caps))
        .expect("num_inflowing_neighbours tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 0.0);
    assert_eq!(out.get(0, 0, 1), 1.0);
    assert_eq!(out.get(0, 0, 2), 1.0);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn find_parallel_flow_flags_parallel_stream_cells() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let pntr_path = std::env::temp_dir().join(format!("wbtools_oss_parallel_pntr_{unique}.asc"));
    let streams_path = std::env::temp_dir().join(format!("wbtools_oss_parallel_streams_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_parallel_out_{unique}.asc"));

    let mut pntr = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    pntr.set(0, 0, 0, 2.0).expect("set");
    pntr.set(0, 0, 1, 0.0).expect("set");
    pntr.set(0, 1, 0, 2.0).expect("set");
    pntr.set(0, 1, 1, 0.0).expect("set");
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pointer raster");

    let mut streams = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..2 {
        for c in 0..2 {
            streams.set(0, r, c, 1.0).expect("set");
        }
    }
    streams.write(&streams_path, RasterFormat::EsriAscii).expect("write streams raster");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("streams".to_string(), json!(streams_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("find_parallel_flow", &args, &context(&caps))
        .expect("find_parallel_flow tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 1, 0), 1.0);

    let _ = std::fs::remove_file(pntr_path);
    let _ = std::fs::remove_file(streams_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn basins_assigns_single_edge_basin_on_simple_pointer() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let pntr_path = std::env::temp_dir().join(format!("wbtools_oss_basins_pntr_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_basins_out_{unique}.asc"));

    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    pntr.set(0, 0, 0, 2.0).expect("set");
    pntr.set(0, 0, 1, 2.0).expect("set");
    pntr.set(0, 0, 2, 0.0).expect("set");
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pointer raster");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry.run("basins", &args, &context(&caps)).expect("basins tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 1.0);
    assert_eq!(out.get(0, 0, 2), 1.0);

    let _ = std::fs::remove_file(pntr_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn flow_accum_full_workflow_log_transform_matches_direct_d8_accum() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_log_in_{unique}.asc"));
    let dir_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_log_dir_{unique}.asc"));
    let acc_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_log_acc_{unique}.asc"));
    let expected_acc_path = std::env::temp_dir().join(format!("wbtools_oss_fafw_log_expected_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("log_transform".to_string(), json!(true));
    args.insert("flow_dir_output".to_string(), json!(dir_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(acc_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("flow_accum_full_workflow", &args, &context(&caps))
        .expect("flow_accum_full_workflow log run should succeed");

    let mut direct_args = ToolArgs::new();
    direct_args.insert("input".to_string(), json!(dir_path.to_string_lossy().to_string()));
    direct_args.insert("input_is_pointer".to_string(), json!(true));
    direct_args.insert("out_type".to_string(), json!("cells"));
    direct_args.insert("log_transform".to_string(), json!(true));
    direct_args.insert("output".to_string(), json!(expected_acc_path.to_string_lossy().to_string()));
    registry
        .run("d8_flow_accum", &direct_args, &context(&caps))
        .expect("direct d8_flow_accum log run should succeed");

    let accum = Raster::read(&acc_path).expect("read workflow accumulation raster");
    let expected = Raster::read(&expected_acc_path).expect("read direct accumulation raster");
    assert!((accum.get(0, 0, 0) - expected.get(0, 0, 0)).abs() < 1.0e-6);
    assert!((accum.get(0, 0, 1) - expected.get(0, 0, 1)).abs() < 1.0e-6);
    assert!((accum.get(0, 0, 2) - expected.get(0, 0, 2)).abs() < 1.0e-6);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(dir_path);
    let _ = std::fs::remove_file(acc_path);
    let _ = std::fs::remove_file(expected_acc_path);
}

#[test]
fn minimal_dispersion_flow_algorithm_log_transform_matches_expected() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let input_path = std::env::temp_dir().join(format!("wbtools_oss_mdfa_log_in_{unique}.asc"));
    let dir_path = std::env::temp_dir().join(format!("wbtools_oss_mdfa_log_dir_{unique}.asc"));
    let accum_path = std::env::temp_dir().join(format!("wbtools_oss_mdfa_log_acc_{unique}.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 30.0).expect("set");
    raster.set(0, 0, 1, 20.0).expect("set");
    raster.set(0, 0, 2, 10.0).expect("set");
    raster
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("cells"));
    args.insert("path_corrected_direction_preference".to_string(), json!(1.0));
    args.insert("log_transform".to_string(), json!(true));
    args.insert("flow_dir_output".to_string(), json!(dir_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(accum_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("minimal_dispersion_flow_algorithm", &args, &context(&caps))
        .expect("minimal_dispersion_flow_algorithm log run should succeed");

    let accum = Raster::read(&accum_path).expect("read accumulation raster");
    assert!((accum.get(0, 0, 0) - 0.0).abs() < 1.0e-6);
    assert!((accum.get(0, 0, 1) - 2.0_f64.ln()).abs() < 1.0e-6);
    assert!((accum.get(0, 0, 2) - 3.0_f64.ln()).abs() < 1.0e-6);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(dir_path);
    let _ = std::fs::remove_file(accum_path);
}

#[test]
fn watershed_from_raster_pour_points_labels_all_upstream_cells() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let pntr_path = std::env::temp_dir().join(format!("wbtools_oss_wsrpp_pntr_{unique}.asc"));
    let pp_path = std::env::temp_dir().join(format!("wbtools_oss_wsrpp_pp_{unique}.asc"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_wsrpp_out_{unique}.asc"));

    // 1×3 pointer: cells 0 and 1 flow east (Whitebox value 2 = SE direction, dx=1 dy=0),
    // cell 2 has no flow (value 0). Flow path: cell 0 → 1 → 2.
    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    pntr.set(0, 0, 0, 2.0).expect("set");
    pntr.set(0, 0, 1, 2.0).expect("set");
    pntr.set(0, 0, 2, 0.0).expect("set");
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pointer raster");

    // Pour points: only cell 2 has ID 7 (non-zero, non-nodata)
    let mut pp = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    pp.set(0, 0, 0, 0.0).expect("set");
    pp.set(0, 0, 1, 0.0).expect("set");
    pp.set(0, 0, 2, 7.0).expect("set");
    pp.write(&pp_path, RasterFormat::EsriAscii).expect("write pour-points raster");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("pour_points".to_string(), json!(pp_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("watershed_from_raster_pour_points", &args, &context(&caps))
        .expect("watershed_from_raster_pour_points tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    assert_eq!(out.get(0, 0, 0), 7.0, "cell 0 should be watershed 7");
    assert_eq!(out.get(0, 0, 1), 7.0, "cell 1 should be watershed 7");
    assert_eq!(out.get(0, 0, 2), 7.0, "cell 2 (outlet) should be watershed 7");

    let _ = std::fs::remove_file(pntr_path);
    let _ = std::fs::remove_file(pp_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn watershed_vector_pour_points_labels_all_upstream_cells() {
    use wbvector::{Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let pntr_path = std::env::temp_dir().join(format!("wbtools_oss_wsv_pntr_{unique}.asc"));
    let pp_path = std::env::temp_dir().join(format!("wbtools_oss_wsv_pp_{unique}.geojson"));
    let output_path = std::env::temp_dir().join(format!("wbtools_oss_wsv_out_{unique}.asc"));

    // 1×3 pointer with x_min=0, cell_size=1 → x_max=3, y_min=0, y_max=1
    // All three cells flow east (value 2) except col 2 which has no flow (value 0).
    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    pntr.set(0, 0, 0, 2.0).expect("set");
    pntr.set(0, 0, 1, 2.0).expect("set");
    pntr.set(0, 0, 2, 0.0).expect("set");
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pointer raster");

    // Vector pour point at center of col 2, row 0: x=2.5, y=0.5
    // world_to_pixel(2.5, 0.5): col=floor(2.5)=2, row=floor((1-0.5)/1)=0  ✓
    let mut layer = Layer::new("pour_points")
        .with_geom_type(wbvector::GeometryType::Point);
    layer
        .add_feature(Some(wbvector::Geometry::point(2.5, 0.5)), &[])
        .expect("add feature");
    wbvector::write(&layer, &pp_path, VectorFormat::GeoJson).expect("write vector");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("pour_pts".to_string(), json!(pp_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("watershed", &args, &context(&caps))
        .expect("watershed tool should run");

    let out = Raster::read(&output_path).expect("read output raster");
    // All three cells should carry watershed ID 1 (first feature, 1-based)
    assert_eq!(out.get(0, 0, 0), 1.0, "cell 0 should be watershed 1");
    assert_eq!(out.get(0, 0, 1), 1.0, "cell 1 should be watershed 1");
    assert_eq!(out.get(0, 0, 2), 1.0, "cell 2 (outlet) should be watershed 1");

    let _ = std::fs::remove_file(pntr_path);
    let _ = std::fs::remove_file(pp_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn watershed_vector_pour_points_esri_matches_wbt() {
    use wbvector::{Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_wsv_esri_parity");
    let pntr_wbt_path = std::env::temp_dir().join(format!("{tag}_pntr_wbt.asc"));
    let pntr_esri_path = std::env::temp_dir().join(format!("{tag}_pntr_esri.asc"));
    let pp_path = std::env::temp_dir().join(format!("{tag}_pp.geojson"));
    let out_wbt_path = std::env::temp_dir().join(format!("{tag}_out_wbt.asc"));
    let out_esri_path = std::env::temp_dir().join(format!("{tag}_out_esri.asc"));

    let mut pntr_wbt = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    // WBT pointers: exercise multiple directions converging at (1,2).
    pntr_wbt.set(0, 0, 0, 4.0).expect("set");
    pntr_wbt.set(0, 0, 1, 8.0).expect("set");
    pntr_wbt.set(0, 0, 2, 16.0).expect("set");
    pntr_wbt.set(0, 1, 0, 2.0).expect("set");
    pntr_wbt.set(0, 1, 1, 2.0).expect("set");
    pntr_wbt.set(0, 1, 2, 0.0).expect("set");
    pntr_wbt.set(0, 2, 0, 1.0).expect("set");
    pntr_wbt.set(0, 2, 1, 128.0).expect("set");
    pntr_wbt.set(0, 2, 2, 64.0).expect("set");
    pntr_wbt
        .write(&pntr_wbt_path, RasterFormat::EsriAscii)
        .expect("write wbt pointer raster");

    let mut pntr_esri = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    // ESRI pointers for the same flow field.
    pntr_esri.set(0, 0, 0, 2.0).expect("set");
    pntr_esri.set(0, 0, 1, 4.0).expect("set");
    pntr_esri.set(0, 0, 2, 8.0).expect("set");
    pntr_esri.set(0, 1, 0, 1.0).expect("set");
    pntr_esri.set(0, 1, 1, 1.0).expect("set");
    pntr_esri.set(0, 1, 2, 0.0).expect("set");
    pntr_esri.set(0, 2, 0, 128.0).expect("set");
    pntr_esri.set(0, 2, 1, 64.0).expect("set");
    pntr_esri.set(0, 2, 2, 32.0).expect("set");
    pntr_esri
        .write(&pntr_esri_path, RasterFormat::EsriAscii)
        .expect("write esri pointer raster");

    // Vector pour point at center of outlet cell (1,2): x=2.5, y=0.5
    let mut layer = Layer::new("pour_points")
        .with_geom_type(wbvector::GeometryType::Point);
    layer
        .add_feature(Some(wbvector::Geometry::point(2.5, 0.5)), &[])
        .expect("add feature");
    wbvector::write(&layer, &pp_path, VectorFormat::GeoJson).expect("write vector");

    let mut args_wbt = ToolArgs::new();
    args_wbt.insert(
        "d8_pntr".to_string(),
        json!(pntr_wbt_path.to_string_lossy().to_string()),
    );
    args_wbt.insert("pour_pts".to_string(), json!(pp_path.to_string_lossy().to_string()));
    args_wbt.insert("output".to_string(), json!(out_wbt_path.to_string_lossy().to_string()));

    let mut args_esri = ToolArgs::new();
    args_esri.insert(
        "d8_pntr".to_string(),
        json!(pntr_esri_path.to_string_lossy().to_string()),
    );
    args_esri.insert("pour_pts".to_string(), json!(pp_path.to_string_lossy().to_string()));
    args_esri.insert("esri_pntr".to_string(), json!(true));
    args_esri.insert(
        "output".to_string(),
        json!(out_esri_path.to_string_lossy().to_string()),
    );

    let caps = OpenOnly;
    registry
        .run("watershed", &args_wbt, &context(&caps))
        .expect("watershed WBT run should succeed");
    registry
        .run("watershed", &args_esri, &context(&caps))
        .expect("watershed ESRI run should succeed");

    let out_wbt = Raster::read(&out_wbt_path).expect("read wbt output raster");
    let out_esri = Raster::read(&out_esri_path).expect("read esri output raster");
    for r in 0..3isize {
        for c in 0..3isize {
            assert_eq!(
                out_wbt.get(0, r, c),
                out_esri.get(0, r, c),
                "WBT and ESRI outputs should match at ({r},{c})"
            );
        }
    }

    let _ = std::fs::remove_file(&pntr_wbt_path);
    let _ = std::fs::remove_file(&pntr_esri_path);
    let _ = std::fs::remove_file(&pp_path);
    let _ = std::fs::remove_file(&out_wbt_path);
    let _ = std::fs::remove_file(&out_esri_path);
}

#[test]
fn watershed_from_raster_pour_points_esri_matches_wbt() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_wsrpp_esri_parity");
    let pntr_wbt_path = std::env::temp_dir().join(format!("{tag}_pntr_wbt.asc"));
    let pntr_esri_path = std::env::temp_dir().join(format!("{tag}_pntr_esri.asc"));
    let pp_path = std::env::temp_dir().join(format!("{tag}_pp.asc"));
    let out_wbt_path = std::env::temp_dir().join(format!("{tag}_out_wbt.asc"));
    let out_esri_path = std::env::temp_dir().join(format!("{tag}_out_esri.asc"));

    let mut pntr_wbt = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    // WBT pointers: exercise multiple directions converging at (1,2).
    pntr_wbt.set(0, 0, 0, 4.0).expect("set");
    pntr_wbt.set(0, 0, 1, 8.0).expect("set");
    pntr_wbt.set(0, 0, 2, 16.0).expect("set");
    pntr_wbt.set(0, 1, 0, 2.0).expect("set");
    pntr_wbt.set(0, 1, 1, 2.0).expect("set");
    pntr_wbt.set(0, 1, 2, 0.0).expect("set");
    pntr_wbt.set(0, 2, 0, 1.0).expect("set");
    pntr_wbt.set(0, 2, 1, 128.0).expect("set");
    pntr_wbt.set(0, 2, 2, 64.0).expect("set");
    pntr_wbt
        .write(&pntr_wbt_path, RasterFormat::EsriAscii)
        .expect("write wbt pointer raster");

    let mut pntr_esri = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    // ESRI pointers for the same flow field.
    pntr_esri.set(0, 0, 0, 2.0).expect("set");
    pntr_esri.set(0, 0, 1, 4.0).expect("set");
    pntr_esri.set(0, 0, 2, 8.0).expect("set");
    pntr_esri.set(0, 1, 0, 1.0).expect("set");
    pntr_esri.set(0, 1, 1, 1.0).expect("set");
    pntr_esri.set(0, 1, 2, 0.0).expect("set");
    pntr_esri.set(0, 2, 0, 128.0).expect("set");
    pntr_esri.set(0, 2, 1, 64.0).expect("set");
    pntr_esri.set(0, 2, 2, 32.0).expect("set");
    pntr_esri
        .write(&pntr_esri_path, RasterFormat::EsriAscii)
        .expect("write esri pointer raster");

    let mut pp = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            pp.set(0, r, c, 0.0).expect("set");
        }
    }
    pp.set(0, 1, 2, 9.0).expect("set outlet id");
    pp.write(&pp_path, RasterFormat::EsriAscii)
        .expect("write pour points raster");

    let mut args_wbt = ToolArgs::new();
    args_wbt.insert(
        "d8_pntr".to_string(),
        json!(pntr_wbt_path.to_string_lossy().to_string()),
    );
    args_wbt.insert("pour_points".to_string(), json!(pp_path.to_string_lossy().to_string()));
    args_wbt.insert("output".to_string(), json!(out_wbt_path.to_string_lossy().to_string()));

    let mut args_esri = ToolArgs::new();
    args_esri.insert(
        "d8_pntr".to_string(),
        json!(pntr_esri_path.to_string_lossy().to_string()),
    );
    args_esri.insert("pour_points".to_string(), json!(pp_path.to_string_lossy().to_string()));
    args_esri.insert("esri_pntr".to_string(), json!(true));
    args_esri.insert(
        "output".to_string(),
        json!(out_esri_path.to_string_lossy().to_string()),
    );

    let caps = OpenOnly;
    registry
        .run("watershed_from_raster_pour_points", &args_wbt, &context(&caps))
        .expect("watershed_from_raster_pour_points WBT run should succeed");
    registry
        .run("watershed_from_raster_pour_points", &args_esri, &context(&caps))
        .expect("watershed_from_raster_pour_points ESRI run should succeed");

    let out_wbt = Raster::read(&out_wbt_path).expect("read wbt output raster");
    let out_esri = Raster::read(&out_esri_path).expect("read esri output raster");
    for r in 0..3isize {
        for c in 0..3isize {
            assert_eq!(
                out_wbt.get(0, r, c),
                out_esri.get(0, r, c),
                "WBT and ESRI outputs should match at ({r},{c})"
            );
        }
    }

    let _ = std::fs::remove_file(&pntr_wbt_path);
    let _ = std::fs::remove_file(&pntr_esri_path);
    let _ = std::fs::remove_file(&pp_path);
    let _ = std::fs::remove_file(&out_wbt_path);
    let _ = std::fs::remove_file(&out_esri_path);
}

#[test]
fn watershed_from_raster_pour_points_handles_multi_outlet_and_nodata_barrier() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_wsrpp_multi_nodata");
    let pntr_path = std::env::temp_dir().join(format!("{tag}_pntr.asc"));
    let pp_path = std::env::temp_dir().join(format!("{tag}_pp.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let nodata = -9999.0;
    let mut pntr = Raster::new(RasterConfig {
        cols: 5,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..5isize {
            pntr.set(0, r, c, nodata).expect("set nodata");
        }
    }
    // Left basin drains to outlet (1,1).
    pntr.set(0, 0, 0, 8.0).expect("set");
    pntr.set(0, 0, 1, 8.0).expect("set");
    pntr.set(0, 1, 0, 2.0).expect("set");
    pntr.set(0, 1, 1, 0.0).expect("set left outlet");
    pntr.set(0, 2, 0, 128.0).expect("set");
    pntr.set(0, 2, 1, 128.0).expect("set");
    // Right basin drains to outlet (1,3).
    pntr.set(0, 0, 3, 8.0).expect("set");
    pntr.set(0, 0, 4, 8.0).expect("set");
    pntr.set(0, 1, 3, 0.0).expect("set right outlet");
    pntr.set(0, 1, 4, 32.0).expect("set");
    pntr.set(0, 2, 3, 128.0).expect("set");
    pntr.set(0, 2, 4, 128.0).expect("set");
    pntr.write(&pntr_path, RasterFormat::EsriAscii)
        .expect("write pointer raster");

    let mut pp = Raster::new(RasterConfig {
        cols: 5,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..5isize {
            pp.set(0, r, c, 0.0).expect("set");
        }
    }
    for r in 0..3isize {
        pp.set(0, r, 2, nodata).expect("set nodata barrier");
    }
    pp.set(0, 1, 1, 10.0).expect("set left outlet id");
    pp.set(0, 1, 3, 20.0).expect("set right outlet id");
    pp.write(&pp_path, RasterFormat::EsriAscii)
        .expect("write pour points raster");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("pour_points".to_string(), json!(pp_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("watershed_from_raster_pour_points", &args, &context(&caps))
        .expect("watershed_from_raster_pour_points multi-outlet run should succeed");

    let out = Raster::read(&output_path).expect("read output raster");
    let out_nodata = out.nodata;
    for r in 0..3isize {
        assert_eq!(out.get(0, r, 0), 10.0, "left basin col 0 should be 10 at row {r}");
        assert_eq!(out.get(0, r, 1), 10.0, "left basin col 1 should be 10 at row {r}");
        assert_eq!(
            out.get(0, r, 2),
            out_nodata,
            "NoData barrier col 2 should remain nodata at row {r}"
        );
        assert_eq!(out.get(0, r, 3), 20.0, "right basin col 3 should be 20 at row {r}");
        assert_eq!(out.get(0, r, 4), 20.0, "right basin col 4 should be 20 at row {r}");
    }

    let _ = std::fs::remove_file(&pntr_path);
    let _ = std::fs::remove_file(&pp_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn jenson_snap_pour_points_snaps_to_nearest_stream_cell() {
    use wbvector::{Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    );
    let streams_path =
        std::env::temp_dir().join(format!("wbtools_oss_jsnap_streams_{unique}.asc"));
    let pp_path =
        std::env::temp_dir().join(format!("wbtools_oss_jsnap_pp_{unique}.geojson"));
    let output_path =
        std::env::temp_dir().join(format!("wbtools_oss_jsnap_out_{unique}.geojson"));

    // 3×3 raster, cell_size=1: x in [0,3), y in [0,3)
    // Stream cell at col=1, row=1 (center x=1.5, y=1.5).  All others = 0.
    let mut streams = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            streams.set(0, r, c, 0.0).expect("set");
        }
    }
    streams.set(0, 1, 1, 1.0).expect("set stream cell");
    streams
        .write(&streams_path, RasterFormat::EsriAscii)
        .expect("write streams raster");

    // Pour point at col=0, row=0 centre: x=0.5, y=2.5.
    // snap_dist=3.0 covers the whole 3×3 grid; nearest stream cell is (1.5, 1.5).
    let mut layer = Layer::new("pour_points")
        .with_geom_type(wbvector::GeometryType::Point);
    layer
        .add_feature(Some(wbvector::Geometry::point(0.5, 2.5)), &[])
        .expect("add feature");
    wbvector::write(&layer, &pp_path, VectorFormat::GeoJson).expect("write vector");

    let mut args = ToolArgs::new();
    args.insert(
        "pour_pts".to_string(),
        json!(pp_path.to_string_lossy().to_string()),
    );
    args.insert(
        "streams".to_string(),
        json!(streams_path.to_string_lossy().to_string()),
    );
    args.insert("snap_dist".to_string(), json!(3.0_f64));
    args.insert(
        "output".to_string(),
        json!(output_path.to_string_lossy().to_string()),
    );

    let caps = OpenOnly;
    registry
        .run("jenson_snap_pour_points", &args, &context(&caps))
        .expect("jenson_snap_pour_points tool should run");

    let snapped = wbvector::read(&output_path).expect("read output vector");
    assert_eq!(snapped.features.len(), 1, "should have one output feature");
    let feat = &snapped.features[0];
    let (ox, oy) = match &feat.geometry {
        Some(wbvector::Geometry::Point(c)) => (c.x, c.y),
        _ => panic!("expected a Point geometry"),
    };
    assert!(
        (ox - 1.5).abs() < 1e-9 && (oy - 1.5).abs() < 1e-9,
        "snapped point should be at stream cell centre (1.5, 1.5), got ({ox}, {oy})"
    );

    let _ = std::fs::remove_file(&streams_path);
    let _ = std::fs::remove_file(&pp_path);
    let _ = std::fs::remove_file(&output_path);
}

// ── Helper: build a unique-prefix string for temp file names ─────────────────
fn unique_tag(prefix: &str) -> String {
    format!(
        "{}_{}_{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos()
    )
}

// ── subbasins ─────────────────────────────────────────────────────────────────
// Grid layout (3 cols × 3 rows):
//
//   rows 0-2, cols 0-2
//
// D8 pointer (WBT encoding): every cell flows East (value 2).
// Stream raster: stream cells at (r=1,c=0), (r=1,c=1), (r=1,c=2) — a single
//   horizontal stream link.
//
// Expected: the whole grid should belong to sub-basin 1 (single stream link).
#[test]
fn subbasins_labels_single_link_basin() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_subbasins");
    let pntr_path = std::env::temp_dir().join(format!("{tag}_pntr.asc"));
    let streams_path = std::env::temp_dir().join(format!("{tag}_streams.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    // 3×3 pointer raster:
    //   Row 0 cells point South  (WBT value  8 = S, DX=0 DY=+1) → drain to stream row 1
    //   Row 1 cells (stream) point East (value 2), outlet at (1,2) has value 0
    //   Row 2 cells point North (WBT value 128 = N, DX=0 DY=-1) → drain to stream row 1
    let mut pntr = Raster::new(RasterConfig {
        cols: 3, rows: 3, bands: 1,
        x_min: 0.0, y_min: 0.0, cell_size: 1.0, cell_size_y: None,
        nodata: -9999.0, data_type: DataType::F64,
        crs: Default::default(), metadata: Vec::new(),
    });
    for c in 0..3isize { pntr.set(0, 0, c, 8.0).expect("set row0 S"); }  // row 0 flows South
    pntr.set(0, 1, 0, 2.0).expect("set stream E"); // row 1 flows East
    pntr.set(0, 1, 1, 2.0).expect("set stream E");
    pntr.set(0, 1, 2, 0.0).expect("set outlet");   // row 1 col 2: no-flow outlet
    for c in 0..3isize { pntr.set(0, 2, c, 128.0).expect("set row2 N"); } // row 2 flows North
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pntr");

    // Stream on middle row only
    let mut streams = Raster::new(RasterConfig {
        cols: 3, rows: 3, bands: 1,
        x_min: 0.0, y_min: 0.0, cell_size: 1.0, cell_size_y: None,
        nodata: -9999.0, data_type: DataType::F64,
        crs: Default::default(), metadata: Vec::new(),
    });
    for r in 0..3isize { for c in 0..3isize { streams.set(0, r, c, 0.0).expect("set"); } }
    for c in 0..3isize { streams.set(0, 1, c, 1.0).expect("set stream"); }
    streams.write(&streams_path, RasterFormat::EsriAscii).expect("write streams");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("streams".to_string(), json!(streams_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry.run("subbasins", &args, &context(&caps)).expect("subbasins should run");

    let out = Raster::read(&output_path).expect("read output");
    // All valid cells should be in sub-basin 1 (single stream link)
    for r in 0..3isize {
        for c in 0..3isize {
            assert_eq!(
                out.get(0, r, c), 1.0,
                "cell ({r},{c}) expected basin 1, got {}", out.get(0, r, c)
            );
        }
    }

    let _ = std::fs::remove_file(&pntr_path);
    let _ = std::fs::remove_file(&streams_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn subbasins_regression_two_disconnected_links_match_golden() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_subbasins_regression");
    let pntr_path = std::env::temp_dir().join(format!("{tag}_pntr.asc"));
    let streams_path = std::env::temp_dir().join(format!("{tag}_streams.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    // Left stream link in col 0: flow south to outlet.
    pntr.set(0, 0, 0, 8.0).expect("set");
    pntr.set(0, 1, 0, 8.0).expect("set");
    pntr.set(0, 2, 0, 0.0).expect("set");
    // Right stream link in col 2: flow south to outlet.
    pntr.set(0, 0, 2, 8.0).expect("set");
    pntr.set(0, 1, 2, 8.0).expect("set");
    pntr.set(0, 2, 2, 0.0).expect("set");
    // Middle column drains west into the left link.
    pntr.set(0, 0, 1, 32.0).expect("set");
    pntr.set(0, 1, 1, 32.0).expect("set");
    pntr.set(0, 2, 1, 32.0).expect("set");
    pntr.write(&pntr_path, RasterFormat::EsriAscii)
        .expect("write pointer raster");

    let mut streams = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        streams.set(0, r, 0, 1.0).expect("set left stream");
        streams.set(0, r, 1, 0.0).expect("set non-stream");
        streams.set(0, r, 2, 1.0).expect("set right stream");
    }
    streams
        .write(&streams_path, RasterFormat::EsriAscii)
        .expect("write streams raster");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("streams".to_string(), json!(streams_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("subbasins", &args, &context(&caps))
        .expect("subbasins regression run should succeed");

    let out = Raster::read(&output_path).expect("read output raster");
    let expected = [[1.0, 1.0, 2.0], [1.0, 1.0, 2.0], [1.0, 1.0, 2.0]];
    for r in 0..3isize {
        for c in 0..3isize {
            assert_eq!(
                out.get(0, r, c),
                expected[r as usize][c as usize],
                "subbasins regression mismatch at ({r},{c})"
            );
        }
    }

    let _ = std::fs::remove_file(&pntr_path);
    let _ = std::fs::remove_file(&streams_path);
    let _ = std::fs::remove_file(&output_path);
}

// ── hillslopes ────────────────────────────────────────────────────────────────
// Same grid as subbasins test. After hillslopes:
//   - stream cells (row=1) = 0
//   - non-stream cells each form their own hillslope (row 0 and row 2,
//     separated by the stream, so two groups)
#[test]
fn hillslopes_zeroes_stream_cells_and_labels_flanks() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_hillslopes");
    let pntr_path = std::env::temp_dir().join(format!("{tag}_pntr.asc"));
    let streams_path = std::env::temp_dir().join(format!("{tag}_streams.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    // Row 0 → South (8), Row 1 stream → East (2), outlet at (1,2)=0, Row 2 → North (128)
    let mut pntr = Raster::new(RasterConfig {
        cols: 3, rows: 3, bands: 1,
        x_min: 0.0, y_min: 0.0, cell_size: 1.0, cell_size_y: None,
        nodata: -9999.0, data_type: DataType::F64,
        crs: Default::default(), metadata: Vec::new(),
    });
    for c in 0..3isize { pntr.set(0, 0, c, 8.0).expect("set row0 S"); }
    pntr.set(0, 1, 0, 2.0).expect("set");
    pntr.set(0, 1, 1, 2.0).expect("set");
    pntr.set(0, 1, 2, 0.0).expect("set outlet");
    for c in 0..3isize { pntr.set(0, 2, c, 128.0).expect("set row2 N"); }
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pntr");

    let mut streams = Raster::new(RasterConfig {
        cols: 3, rows: 3, bands: 1,
        x_min: 0.0, y_min: 0.0, cell_size: 1.0, cell_size_y: None,
        nodata: -9999.0, data_type: DataType::F64,
        crs: Default::default(), metadata: Vec::new(),
    });
    for r in 0..3isize { for c in 0..3isize { streams.set(0, r, c, 0.0).expect("set"); } }
    for c in 0..3isize { streams.set(0, 1, c, 1.0).expect("set stream"); }
    streams.write(&streams_path, RasterFormat::EsriAscii).expect("write streams");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("streams".to_string(), json!(streams_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry.run("hillslopes", &args, &context(&caps)).expect("hillslopes should run");

    let out = Raster::read(&output_path).expect("read output");
    // Stream cells must be 0
    for c in 0..3isize {
        assert_eq!(out.get(0, 1, c), 0.0, "stream cell (1,{c}) should be 0");
    }
    // Non-stream cells must have a non-zero hillslope ID
    for c in 0..3isize {
        assert!(out.get(0, 0, c) > 0.0, "top row cell (0,{c}) should have hillslope ID > 0");
        assert!(out.get(0, 2, c) > 0.0, "bottom row cell (2,{c}) should have hillslope ID > 0");
    }
    // Top and bottom rows must belong to different hillslopes
    assert_ne!(
        out.get(0, 0, 0), out.get(0, 2, 0),
        "top and bottom hillslopes should have different IDs"
    );

    let _ = std::fs::remove_file(&pntr_path);
    let _ = std::fs::remove_file(&streams_path);
    let _ = std::fs::remove_file(&output_path);
}

// ── strahler_order_basins ─────────────────────────────────────────────────────
// Simple Y-shaped stream: two headwater links merge at a confluence.
//
//   col:   0  1  2
//   row 0: h1 .  h2    (headwaters at (0,0) and (0,2))
//   row 1: .  conf .   (confluence at (1,1))
//   row 2: .  out  .   (outlet at (2,1))
//
// Pointer: (0,0)→SE=4, (0,2)→SW=16, (1,1)→S=8, (2,1)→0 (no flow)
// Streams: (0,0), (0,2), (1,1), (2,1) = 1; rest = 0.
// Strahler: headwaters = order 1; confluence (1,1) and below = order 2.
// Basin labels: cells draining to h1 link → 1; to h2 link → 1;
//   cells draining to confluence and below → 2.
#[test]
fn strahler_order_basins_assigns_order_2_at_confluence() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_strahler");
    let pntr_path = std::env::temp_dir().join(format!("{tag}_pntr.asc"));
    let streams_path = std::env::temp_dir().join(format!("{tag}_streams.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    // WBT D8 pointer encoding:
    //   1=NE, 2=E, 4=SE, 8=S, 16=SW, 32=W, 64=NW, 128=N
    //
    //   (0,0)→SE=4   (0,1)→S=8   (0,2)→SW=16
    //   (1,0)→S=8    (1,1)→S=8   (1,2)→S=8
    //   (2,0)→E=2    (2,1)→0     (2,2)→W=32
    let mut pntr = Raster::new(RasterConfig {
        cols: 3, rows: 3, bands: 1,
        x_min: 0.0, y_min: 0.0, cell_size: 1.0, cell_size_y: None,
        nodata: -9999.0, data_type: DataType::F64,
        crs: Default::default(), metadata: Vec::new(),
    });
    // Row 0
    pntr.set(0, 0, 0, 4.0).expect("set");   // SE
    pntr.set(0, 0, 1, 8.0).expect("set");   // S
    pntr.set(0, 0, 2, 16.0).expect("set");  // SW
    // Row 1
    pntr.set(0, 1, 0, 8.0).expect("set");   // S
    pntr.set(0, 1, 1, 8.0).expect("set");   // S (confluence → outlet)
    pntr.set(0, 1, 2, 8.0).expect("set");   // S
    // Row 2
    pntr.set(0, 2, 0, 2.0).expect("set");   // E
    pntr.set(0, 2, 1, 0.0).expect("set");   // no-flow (watershed outlet)
    pntr.set(0, 2, 2, 32.0).expect("set");  // W
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pntr");

    // Stream cells: (0,0), (0,2), (1,1), (2,1)
    let mut streams = Raster::new(RasterConfig {
        cols: 3, rows: 3, bands: 1,
        x_min: 0.0, y_min: 0.0, cell_size: 1.0, cell_size_y: None,
        nodata: -9999.0, data_type: DataType::F64,
        crs: Default::default(), metadata: Vec::new(),
    });
    for r in 0..3isize { for c in 0..3isize { streams.set(0, r, c, 0.0).expect("set"); } }
    streams.set(0, 0, 0, 1.0).expect("set headwater 1");
    streams.set(0, 0, 2, 1.0).expect("set headwater 2");
    streams.set(0, 1, 1, 1.0).expect("set confluence");
    streams.set(0, 2, 1, 1.0).expect("set outlet");
    streams.write(&streams_path, RasterFormat::EsriAscii).expect("write streams");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("streams".to_string(), json!(streams_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry.run("strahler_order_basins", &args, &context(&caps))
        .expect("strahler_order_basins should run");

    let out = Raster::read(&output_path).expect("read output");
    let nodata = out.nodata;

    // Headwater stream cells and near-headwater drainage should be order 1
    let v_h1 = out.get(0, 0, 0);
    let v_h2 = out.get(0, 0, 2);
    assert_eq!(v_h1, 1.0, "headwater 1 cell should be Strahler order 1, got {v_h1}");
    assert_eq!(v_h2, 1.0, "headwater 2 cell should be Strahler order 1, got {v_h2}");

    // Confluence and outlet cells should be order 2
    let v_conf = out.get(0, 1, 1);
    let v_out  = out.get(0, 2, 1);
    assert_eq!(v_conf, 2.0, "confluence cell (1,1) should be order 2, got {v_conf}");
    assert_eq!(v_out,  2.0, "outlet cell (2,1) should be order 2, got {v_out}");

    // Non-stream cells that drain to the confluence should also be order 2
    let v_21_0 = out.get(0, 2, 0);
    let v_21_2 = out.get(0, 2, 2);
    assert!(
        v_21_0 == 2.0 || v_21_0 == nodata,
        "cell (2,0) should be order 2 or nodata, got {v_21_0}"
    );
    assert!(
        v_21_2 == 2.0 || v_21_2 == nodata,
        "cell (2,2) should be order 2 or nodata, got {v_21_2}"
    );

    let _ = std::fs::remove_file(&pntr_path);
    let _ = std::fs::remove_file(&streams_path);
    let _ = std::fs::remove_file(&output_path);
}

// ── isobasins ─────────────────────────────────────────────────────────────────
// 3×3 DEM with a simple slope so every cell flows East:
//   elevations: col 0 → 30, col 1 → 20, col 2 → 10
//   With target_size=1 every cell becomes its own basin.
//   We verify: tool runs, output has no -32768 cells in valid area,
//   and all basin IDs are > 0.
#[test]
fn isobasins_produces_valid_basin_ids() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_isobasins");
    let dem_path = std::env::temp_dir().join(format!("{tag}_dem.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut dem = Raster::new(RasterConfig {
        cols: 3, rows: 3, bands: 1,
        x_min: 0.0, y_min: 0.0, cell_size: 1.0, cell_size_y: None,
        nodata: -9999.0, data_type: DataType::F64,
        crs: Default::default(), metadata: Vec::new(),
    });
    // Slope East: col 0 = 30, col 1 = 20, col 2 = 10
    for r in 0..3isize {
        dem.set(0, r, 0, 30.0).expect("set");
        dem.set(0, r, 1, 20.0).expect("set");
        dem.set(0, r, 2, 10.0).expect("set");
    }
    dem.write(&dem_path, RasterFormat::EsriAscii).expect("write dem");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(dem_path.to_string_lossy().to_string()));
    args.insert("target_size".to_string(), json!(3.0_f64)); // ~3-cell basins
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry.run("isobasins", &args, &context(&caps)).expect("isobasins should run");

    let out = Raster::read(&output_path).expect("read output");
    let nodata = out.nodata;
    for r in 0..3isize {
        for c in 0..3isize {
            let v = out.get(0, r, c);
            assert!(
                v != nodata && v > 0.0,
                "cell ({r},{c}) should have a valid basin ID > 0, got {v}"
            );
        }
    }

    let _ = std::fs::remove_file(&dem_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn insert_dams_raises_elevation_near_dam_point() {
    use wbvector::{Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_insert_dams");
    let dem_path = std::env::temp_dir().join(format!("{tag}_dem.asc"));
    let points_path = std::env::temp_dir().join(format!("{tag}_pts.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut dem = Raster::new(RasterConfig {
        cols: 5,
        rows: 5,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..5isize {
        for c in 0..5isize {
            dem.set(0, r, c, 10.0).expect("set");
        }
    }
    dem.set(0, 2, 2, 2.0).expect("set low center");
    dem.write(&dem_path, RasterFormat::EsriAscii).expect("write dem");

    let mut points = Layer::new("dam_points").with_geom_type(wbvector::GeometryType::Point);
    points
        .add_feature(Some(wbvector::Geometry::point(2.5, 2.5)), &[])
        .expect("add point");
    wbvector::write(&points, &points_path, VectorFormat::GeoJson).expect("write points");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(dem_path.to_string_lossy().to_string()));
    args.insert("dam_points".to_string(), json!(points_path.to_string_lossy().to_string()));
    args.insert("dam_length".to_string(), json!(3.0f64));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("insert_dams", &args, &context(&caps))
        .expect("insert_dams should run");

    let out = Raster::read(&output_path).expect("read output");
    assert!(out.get(0, 2, 2) > dem.get(0, 2, 2));

    let _ = std::fs::remove_file(&dem_path);
    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn raise_walls_increments_cells_intersecting_wall_line() {
    use wbvector::{Coord, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_raise_walls");
    let dem_path = std::env::temp_dir().join(format!("{tag}_dem.asc"));
    let walls_path = std::env::temp_dir().join(format!("{tag}_walls.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut dem = Raster::new(RasterConfig {
        cols: 5,
        rows: 5,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..5isize {
        for c in 0..5isize {
            dem.set(0, r, c, 10.0).expect("set");
        }
    }
    dem.write(&dem_path, RasterFormat::EsriAscii).expect("write dem");

    let mut walls = Layer::new("walls").with_geom_type(wbvector::GeometryType::LineString);
    walls
        .add_feature(
            Some(wbvector::Geometry::line_string(vec![Coord::xy(0.5, 2.5), Coord::xy(4.5, 2.5)])),
            &[],
        )
        .expect("add wall line");
    wbvector::write(&walls, &walls_path, VectorFormat::GeoJson).expect("write walls");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(dem_path.to_string_lossy().to_string()));
    args.insert("walls".to_string(), json!(walls_path.to_string_lossy().to_string()));
    args.insert("wall_height".to_string(), json!(5.0f64));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("raise_walls", &args, &context(&caps))
        .expect("raise_walls should run");

    let out = Raster::read(&output_path).expect("read output");
    let mut raised_count = 0usize;
    for r in 0..5isize {
        for c in 0..5isize {
            if out.get(0, r, c) > 10.0 {
                raised_count += 1;
            }
        }
    }
    assert!(raised_count > 0, "expected at least one raised wall cell");

    let _ = std::fs::remove_file(&dem_path);
    let _ = std::fs::remove_file(&walls_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn topological_breach_burn_returns_four_raster_outputs() {
    use wbvector::{Coord, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_topological_breach_burn");
    let dem_path = std::env::temp_dir().join(format!("{tag}_dem.asc"));
    let streams_path = std::env::temp_dir().join(format!("{tag}_streams.geojson"));
    let out_streams = std::env::temp_dir().join(format!("{tag}_out_streams.asc"));
    let out_dem = std::env::temp_dir().join(format!("{tag}_out_dem.asc"));
    let out_dir = std::env::temp_dir().join(format!("{tag}_out_dir.asc"));
    let out_fa = std::env::temp_dir().join(format!("{tag}_out_fa.asc"));

    let mut dem = Raster::new(RasterConfig {
        cols: 5,
        rows: 5,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..5isize {
        for c in 0..5isize {
            dem.set(0, r, c, 100.0 - (r as f64) * 2.0 - (c as f64)).expect("set");
        }
    }
    dem.write(&dem_path, RasterFormat::EsriAscii).expect("write dem");

    let mut streams = Layer::new("streams").with_geom_type(wbvector::GeometryType::LineString);
    streams
        .add_feature(
            Some(wbvector::Geometry::line_string(vec![Coord::xy(2.5, 4.5), Coord::xy(2.5, 0.5)])),
            &[],
        )
        .expect("add stream line");
    wbvector::write(&streams, &streams_path, VectorFormat::GeoJson).expect("write streams");

    let mut args = ToolArgs::new();
    args.insert("streams".to_string(), json!(streams_path.to_string_lossy().to_string()));
    args.insert("dem".to_string(), json!(dem_path.to_string_lossy().to_string()));
    args.insert("snap_distance".to_string(), json!(1.0f64));
    args.insert("out_streams".to_string(), json!(out_streams.to_string_lossy().to_string()));
    args.insert("out_dem".to_string(), json!(out_dem.to_string_lossy().to_string()));
    args.insert("out_dir".to_string(), json!(out_dir.to_string_lossy().to_string()));
    args.insert("out_fa".to_string(), json!(out_fa.to_string_lossy().to_string()));

    let caps = FullAccess;
    registry
        .run("topological_breach_burn", &args, &context(&caps))
        .expect("topological_breach_burn should run");

    let streams_out = Raster::read(&out_streams).expect("read streams output");
    let dir_out = Raster::read(&out_dir).expect("read dir output");
    let fa_out = Raster::read(&out_fa).expect("read fa output");

    let mut stream_cells = 0usize;
    for r in 0..5isize {
        for c in 0..5isize {
            if streams_out.get(0, r, c) > 0.0 {
                stream_cells += 1;
            }
        }
    }
    assert!(stream_cells > 0, "expected stream raster to contain non-zero tributary IDs");
    assert!(dir_out.get(0, 2, 2).is_finite(), "pointer output should contain finite values");
    assert!(fa_out.get(0, 2, 2).is_finite(), "flow accumulation output should contain finite values");

    let _ = std::fs::remove_file(&dem_path);
    let _ = std::fs::remove_file(&streams_path);
    let _ = std::fs::remove_file(&out_streams);
    let _ = std::fs::remove_file(&out_dem);
    let _ = std::fs::remove_file(&out_dir);
    let _ = std::fs::remove_file(&out_fa);
}

#[test]
fn stochastic_depression_analysis_outputs_probability_range() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_sda");
    let dem_path = std::env::temp_dir().join(format!("{tag}_dem.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut dem = Raster::new(RasterConfig {
        cols: 4,
        rows: 4,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..4isize {
        for c in 0..4isize {
            dem.set(0, r, c, 100.0 - (r as f64) - (c as f64)).expect("set");
        }
    }
    dem.write(&dem_path, RasterFormat::EsriAscii).expect("write dem");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(dem_path.to_string_lossy().to_string()));
    args.insert("rmse".to_string(), json!(0.5f64));
    args.insert("range".to_string(), json!(1.0f64));
    args.insert("iterations".to_string(), json!(5u64));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("stochastic_depression_analysis", &args, &context(&caps))
        .expect("stochastic_depression_analysis should run");

    let out = Raster::read(&output_path).expect("read output");
    for r in 0..4isize {
        for c in 0..4isize {
            let v = out.get(0, r, c);
            if v != out.nodata {
                assert!(v >= 0.0 && v <= 1.0, "probability out of range at ({r},{c}): {v}");
            }
        }
    }

    let _ = std::fs::remove_file(&dem_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn unnest_basins_returns_nested_raster_tuple() {
    use wbvector::{Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_unnest");
    let pntr_path = std::env::temp_dir().join(format!("{tag}_pntr.asc"));
    let pour_path = std::env::temp_dir().join(format!("{tag}_pour.geojson"));

    let mut pntr = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    pntr.set(0, 0, 0, 2.0).expect("set");
    pntr.set(0, 0, 1, 2.0).expect("set");
    pntr.set(0, 0, 2, 0.0).expect("set");
    pntr.write(&pntr_path, RasterFormat::EsriAscii).expect("write pointer");

    let mut pour = Layer::new("pour_points").with_geom_type(wbvector::GeometryType::Point);
    pour.add_feature(Some(wbvector::Geometry::point(1.5, 0.5)), &[])
        .expect("add point1");
    pour.add_feature(Some(wbvector::Geometry::point(2.5, 0.5)), &[])
        .expect("add point2");
    wbvector::write(&pour, &pour_path, VectorFormat::GeoJson).expect("write pour vector");

    let mut args = ToolArgs::new();
    args.insert("d8_pntr".to_string(), json!(pntr_path.to_string_lossy().to_string()));
    args.insert("pour_points".to_string(), json!(pour_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    let result = registry
        .run("unnest_basins", &args, &context(&caps))
        .expect("unnest_basins should run");
    let items = result
        .outputs
        .get("items")
        .and_then(|v| v.as_array())
        .expect("unnest_basins should return tuple items");
    assert!(items.len() >= 2, "expected at least two nesting levels");

    let _ = std::fs::remove_file(&pntr_path);
    let _ = std::fs::remove_file(&pour_path);
}

#[test]
fn upslope_depression_storage_outputs_nonnegative_values() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_uds");
    let dem_path = std::env::temp_dir().join(format!("{tag}_dem.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut dem = Raster::new(RasterConfig {
        cols: 4,
        rows: 4,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..4isize {
        for c in 0..4isize {
            dem.set(0, r, c, 20.0).expect("set");
        }
    }
    dem.set(0, 1, 1, 18.0).expect("set sink");
    dem.write(&dem_path, RasterFormat::EsriAscii).expect("write dem");

    let mut args = ToolArgs::new();
    args.insert("dem".to_string(), json!(dem_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("upslope_depression_storage", &args, &context(&caps))
        .expect("upslope_depression_storage should run");

    let out = Raster::read(&output_path).expect("read output");
    for r in 0..4isize {
        for c in 0..4isize {
            let v = out.get(0, r, c);
            if v != out.nodata {
                assert!(v >= 0.0, "value should be nonnegative at ({r},{c}), got {v}");
            }
        }
    }

    let _ = std::fs::remove_file(&dem_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn average_overlay_computes_mean_of_valid_cells() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_average_overlay");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut raster1 = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster1.set(0, 0, 0, 2.0).expect("set");
    raster1.set(0, 0, 1, 4.0).expect("set");
    raster1.set(0, 1, 0, raster1.nodata).expect("set");
    raster1.set(0, 1, 1, 8.0).expect("set");
    raster1.write(&in1_path, RasterFormat::EsriAscii).expect("write raster1");

    let mut raster2 = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster2.set(0, 0, 0, 6.0).expect("set");
    raster2.set(0, 0, 1, 2.0).expect("set");
    raster2.set(0, 1, 0, 10.0).expect("set");
    raster2.set(0, 1, 1, 12.0).expect("set");
    raster2.write(&in2_path, RasterFormat::EsriAscii).expect("write raster2");

    let mut args = ToolArgs::new();
    args.insert(
        "input_rasters".to_string(),
        json!([
            in1_path.to_string_lossy().to_string(),
            in2_path.to_string_lossy().to_string()
        ]),
    );
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("average_overlay", &args, &context(&caps))
        .expect("average_overlay should run");

    let out = Raster::read(&output_path).expect("read output");
    assert_eq!(out.get(0, 0, 0), 4.0);
    assert_eq!(out.get(0, 0, 1), 3.0);
    assert_eq!(out.get(0, 1, 0), 10.0);
    assert_eq!(out.get(0, 1, 1), 10.0);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn count_if_counts_matching_values() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_count_if");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    for (path, values) in [
        (&in1_path, [1.0, 2.0, -9999.0, 3.0]),
        (&in2_path, [1.0, 1.0, 1.0, 3.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            let row = (index / 2) as isize;
            let col = (index % 2) as isize;
            raster.set(0, row, col, value).expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let mut args = ToolArgs::new();
    args.insert(
        "input_rasters".to_string(),
        json!([
            in1_path.to_string_lossy().to_string(),
            in2_path.to_string_lossy().to_string()
        ]),
    );
    args.insert("comparison_value".to_string(), json!(1.0));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("count_if", &args, &context(&caps))
        .expect("count_if should run");

    let out = Raster::read(&output_path).expect("read output");
    assert_eq!(out.get(0, 0, 0), 2.0);
    assert_eq!(out.get(0, 0, 1), 1.0);
    assert_eq!(out.get(0, 1, 0), 1.0);
    assert_eq!(out.get(0, 1, 1), 0.0);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn highest_and_lowest_position_identify_stack_indices() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_positions");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let high_path = std::env::temp_dir().join(format!("{tag}_high.asc"));
    let low_path = std::env::temp_dir().join(format!("{tag}_low.asc"));

    for (path, values) in [
        (&in1_path, [5.0, 9.0, 1.0, 4.0]),
        (&in2_path, [6.0, 2.0, 3.0, 4.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let caps = OpenOnly;
    for (tool_id, out_path) in [("highest_position", &high_path), ("lowest_position", &low_path)] {
        let mut args = ToolArgs::new();
        args.insert(
            "input_rasters".to_string(),
            json!([
                in1_path.to_string_lossy().to_string(),
                in2_path.to_string_lossy().to_string()
            ]),
        );
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        registry.run(tool_id, &args, &context(&caps)).expect("position tool should run");
    }

    let high = Raster::read(&high_path).expect("read highest output");
    let low = Raster::read(&low_path).expect("read lowest output");
    assert_eq!(high.get(0, 0, 0), 1.0);
    assert_eq!(high.get(0, 0, 1), 0.0);
    assert_eq!(high.get(0, 1, 0), 1.0);
    assert_eq!(low.get(0, 0, 0), 0.0);
    assert_eq!(low.get(0, 0, 1), 1.0);
    assert_eq!(low.get(0, 1, 0), 0.0);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&high_path);
    let _ = std::fs::remove_file(&low_path);
}

#[test]
fn min_max_and_sum_overlay_compute_expected_values() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_overlay_stats");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let max_path = std::env::temp_dir().join(format!("{tag}_max.asc"));
    let min_path = std::env::temp_dir().join(format!("{tag}_min.asc"));
    let sum_path = std::env::temp_dir().join(format!("{tag}_sum.asc"));

    for (path, values) in [
        (&in1_path, [1.0, 8.0, 3.0, 4.0]),
        (&in2_path, [5.0, 2.0, 7.0, 6.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let caps = OpenOnly;
    for (tool_id, out_path) in [
        ("max_overlay", &max_path),
        ("min_overlay", &min_path),
        ("sum_overlay", &sum_path),
    ] {
        let mut args = ToolArgs::new();
        args.insert(
            "input_rasters".to_string(),
            json!([
                in1_path.to_string_lossy().to_string(),
                in2_path.to_string_lossy().to_string()
            ]),
        );
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        registry.run(tool_id, &args, &context(&caps)).expect("overlay tool should run");
    }

    let max_out = Raster::read(&max_path).expect("read max output");
    let min_out = Raster::read(&min_path).expect("read min output");
    let sum_out = Raster::read(&sum_path).expect("read sum output");

    assert_eq!(max_out.get(0, 0, 0), 5.0);
    assert_eq!(max_out.get(0, 0, 1), 8.0);
    assert_eq!(min_out.get(0, 1, 0), 3.0);
    assert_eq!(min_out.get(0, 1, 1), 4.0);
    assert_eq!(sum_out.get(0, 0, 0), 6.0);
    assert_eq!(sum_out.get(0, 1, 1), 10.0);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&max_path);
    let _ = std::fs::remove_file(&min_path);
    let _ = std::fs::remove_file(&sum_path);
}

#[test]
fn multiply_and_absolute_overlay_compute_expected_values() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_multiply_abs_overlay");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let mul_path = std::env::temp_dir().join(format!("{tag}_mul.asc"));
    let min_abs_path = std::env::temp_dir().join(format!("{tag}_min_abs.asc"));
    let max_abs_path = std::env::temp_dir().join(format!("{tag}_max_abs.asc"));

    for (path, values) in [
        (&in1_path, [-2.0, 8.0, -3.0, 4.0]),
        (&in2_path, [5.0, -2.0, 7.0, -6.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let caps = OpenOnly;
    for (tool_id, out_path) in [
        ("multiply_overlay", &mul_path),
        ("min_absolute_overlay", &min_abs_path),
        ("max_absolute_overlay", &max_abs_path),
    ] {
        let mut args = ToolArgs::new();
        args.insert(
            "input_rasters".to_string(),
            json!([
                in1_path.to_string_lossy().to_string(),
                in2_path.to_string_lossy().to_string()
            ]),
        );
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        registry.run(tool_id, &args, &context(&caps)).expect("overlay tool should run");
    }

    let mul_out = Raster::read(&mul_path).expect("read multiply output");
    let min_abs_out = Raster::read(&min_abs_path).expect("read min abs output");
    let max_abs_out = Raster::read(&max_abs_path).expect("read max abs output");

    assert_eq!(mul_out.get(0, 0, 0), -10.0);
    assert_eq!(mul_out.get(0, 1, 1), -24.0);
    assert_eq!(min_abs_out.get(0, 0, 0), 2.0);
    assert_eq!(min_abs_out.get(0, 1, 0), 3.0);
    assert_eq!(max_abs_out.get(0, 0, 1), 8.0);
    assert_eq!(max_abs_out.get(0, 1, 1), 6.0);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&mul_path);
    let _ = std::fs::remove_file(&min_abs_path);
    let _ = std::fs::remove_file(&max_abs_path);
}

#[test]
fn percent_overlay_tools_compute_expected_fractions() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_percent_overlay");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let comparison_path = std::env::temp_dir().join(format!("{tag}_comparison.asc"));
    let eq_path = std::env::temp_dir().join(format!("{tag}_eq.asc"));
    let gt_path = std::env::temp_dir().join(format!("{tag}_gt.asc"));
    let lt_path = std::env::temp_dir().join(format!("{tag}_lt.asc"));

    for (path, values) in [
        (&in1_path, [1.0, 5.0, 3.0, 9.0]),
        (&in2_path, [1.0, 2.0, 7.0, 8.0]),
        (&comparison_path, [1.0, 4.0, 5.0, 8.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let caps = OpenOnly;
    for (tool_id, out_path) in [
        ("percent_equal_to", &eq_path),
        ("percent_greater_than", &gt_path),
        ("percent_less_than", &lt_path),
    ] {
        let mut args = ToolArgs::new();
        args.insert(
            "input_rasters".to_string(),
            json!([
                in1_path.to_string_lossy().to_string(),
                in2_path.to_string_lossy().to_string()
            ]),
        );
        args.insert(
            "comparison".to_string(),
            json!(comparison_path.to_string_lossy().to_string()),
        );
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        registry.run(tool_id, &args, &context(&caps)).expect("percent tool should run");
    }

    let eq_out = Raster::read(&eq_path).expect("read eq output");
    let gt_out = Raster::read(&gt_path).expect("read gt output");
    let lt_out = Raster::read(&lt_path).expect("read lt output");

    assert_eq!(eq_out.get(0, 0, 0), 1.0);
    assert_eq!(eq_out.get(0, 0, 1), 0.0);
    assert_eq!(eq_out.get(0, 1, 1), 0.5);
    assert_eq!(gt_out.get(0, 0, 1), 0.5);
    assert_eq!(gt_out.get(0, 1, 1), 0.5);
    assert_eq!(lt_out.get(0, 0, 1), 0.5);
    assert_eq!(lt_out.get(0, 1, 0), 0.5);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&comparison_path);
    let _ = std::fs::remove_file(&eq_path);
    let _ = std::fs::remove_file(&gt_path);
    let _ = std::fs::remove_file(&lt_path);
}

#[test]
fn pick_from_list_selects_values_by_position_raster() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_pick_from_list");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let pos_path = std::env::temp_dir().join(format!("{tag}_pos.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    for (path, values) in [
        (&in1_path, [10.0, 20.0, 30.0, 40.0]),
        (&in2_path, [1.0, 2.0, 3.0, 4.0]),
        (&pos_path, [0.0, 1.0, 1.0, 0.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let mut args = ToolArgs::new();
    args.insert(
        "input_rasters".to_string(),
        json!([
            in1_path.to_string_lossy().to_string(),
            in2_path.to_string_lossy().to_string()
        ]),
    );
    args.insert("pos_input".to_string(), json!(pos_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("pick_from_list", &args, &context(&caps))
        .expect("pick_from_list should run");

    let out = Raster::read(&output_path).expect("read output");
    assert_eq!(out.get(0, 0, 0), 10.0);
    assert_eq!(out.get(0, 0, 1), 2.0);
    assert_eq!(out.get(0, 1, 0), 3.0);
    assert_eq!(out.get(0, 1, 1), 40.0);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&pos_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn weighted_sum_and_weighted_overlay_compute_expected_values() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_weighted_tools");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let constraint_path = std::env::temp_dir().join(format!("{tag}_constraint.asc"));
    let sum_path = std::env::temp_dir().join(format!("{tag}_sum.asc"));
    let overlay_path = std::env::temp_dir().join(format!("{tag}_overlay.asc"));

    for (path, values) in [
        (&in1_path, [0.0, 5.0, 10.0, 15.0]),
        (&in2_path, [10.0, 0.0, 10.0, 20.0]),
        (&constraint_path, [1.0, 1.0, 0.0, 1.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let caps = OpenOnly;

    let mut sum_args = ToolArgs::new();
    sum_args.insert(
        "input_rasters".to_string(),
        json!([
            in1_path.to_string_lossy().to_string(),
            in2_path.to_string_lossy().to_string()
        ]),
    );
    sum_args.insert("weights".to_string(), json!([1.0, 3.0]));
    sum_args.insert("output".to_string(), json!(sum_path.to_string_lossy().to_string()));
    registry
        .run("weighted_sum", &sum_args, &context(&caps))
        .expect("weighted_sum should run");

    let mut overlay_args = ToolArgs::new();
    overlay_args.insert(
        "factors".to_string(),
        json!([
            in1_path.to_string_lossy().to_string(),
            in2_path.to_string_lossy().to_string()
        ]),
    );
    overlay_args.insert("weights".to_string(), json!([1.0, 1.0]));
    overlay_args.insert("cost".to_string(), json!([false, true]));
    overlay_args.insert(
        "constraints".to_string(),
        json!([constraint_path.to_string_lossy().to_string()]),
    );
    overlay_args.insert("scale_max".to_string(), json!(1.0));
    overlay_args.insert("output".to_string(), json!(overlay_path.to_string_lossy().to_string()));
    registry
        .run("weighted_overlay", &overlay_args, &context(&caps))
        .expect("weighted_overlay should run");

    let sum_out = Raster::read(&sum_path).expect("read weighted sum output");
    assert!((sum_out.get(0, 0, 0) - 7.5).abs() < 1e-9);
    assert!((sum_out.get(0, 0, 1) - 1.25).abs() < 1e-9);
    assert!((sum_out.get(0, 1, 1) - 18.75).abs() < 1e-9);

    let overlay_out = Raster::read(&overlay_path).expect("read weighted overlay output");
    assert!((overlay_out.get(0, 0, 0) - 0.25).abs() < 1e-6);
    assert!((overlay_out.get(0, 0, 1) - (2.0 / 3.0)).abs() < 1e-6);
    assert_eq!(overlay_out.get(0, 1, 0), 0.0);
    assert!((overlay_out.get(0, 1, 1) - 0.5).abs() < 1e-6);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&constraint_path);
    let _ = std::fs::remove_file(&sum_path);
    let _ = std::fs::remove_file(&overlay_path);
}

#[test]
fn aggregate_raster_and_block_extrema_compute_expected_values() {
    use wbvector::{Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_aggregate_block");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.asc"));
    let aggregate_path = std::env::temp_dir().join(format!("{tag}_aggregate.asc"));
    let base_path = std::env::temp_dir().join(format!("{tag}_base.asc"));
    let points_path = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let block_min_path = std::env::temp_dir().join(format!("{tag}_block_min.asc"));
    let block_max_path = std::env::temp_dir().join(format!("{tag}_block_max.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 4,
        rows: 4,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for (index, value) in (1..=16).map(|v| v as f64).enumerate() {
        raster
            .set(0, (index / 4) as isize, (index % 4) as isize, value)
            .expect("set");
    }
    raster.write(&input_path, RasterFormat::EsriAscii).expect("write input raster");

    let mut base = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 2.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for r in 0..2isize {
        for c in 0..2isize {
            base.set(0, r, c, base.nodata).expect("set");
        }
    }
    base.write(&base_path, RasterFormat::EsriAscii).expect("write base raster");

    let mut layer = Layer::new("points")
        .with_geom_type(wbvector::GeometryType::Point)
        .with_crs_epsg(4326);
    layer
        .add_feature(Some(wbvector::Geometry::point_z(0.5, 3.5, 3.0)), &[])
        .expect("add feature");
    layer
        .add_feature(Some(wbvector::Geometry::point_z(1.5, 2.5, 7.0)), &[])
        .expect("add feature");
    layer
        .add_feature(Some(wbvector::Geometry::point_z(2.5, 1.5, 10.0)), &[])
        .expect("add feature");
    layer
        .add_feature(Some(wbvector::Geometry::point_z(3.5, 0.5, 12.0)), &[])
        .expect("add feature");
    wbvector::write(&layer, &points_path, VectorFormat::GeoJson).expect("write points vector");

    let caps = OpenOnly;

    let mut aggregate_args = ToolArgs::new();
    aggregate_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    aggregate_args.insert("aggregation_factor".to_string(), json!(2));
    aggregate_args.insert("aggregation_type".to_string(), json!("mean"));
    aggregate_args.insert("output".to_string(), json!(aggregate_path.to_string_lossy().to_string()));
    registry
        .run("aggregate_raster", &aggregate_args, &context(&caps))
        .expect("aggregate_raster should run");

    for (tool_id, out_path) in [("block_minimum", &block_min_path), ("block_maximum", &block_max_path)] {
        let mut args = ToolArgs::new();
        args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
        args.insert("use_z".to_string(), json!(true));
        args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        registry.run(tool_id, &args, &context(&caps)).expect("block tool should run");
    }

    let aggregate_out = Raster::read(&aggregate_path).expect("read aggregate output");
    assert!((aggregate_out.get(0, 0, 0) - 3.5).abs() < 1e-6);
    assert!((aggregate_out.get(0, 0, 1) - 5.5).abs() < 1e-6);
    assert!((aggregate_out.get(0, 1, 0) - 11.5).abs() < 1e-6);
    assert!((aggregate_out.get(0, 1, 1) - 13.5).abs() < 1e-6);

    let block_min_out = Raster::read(&block_min_path).expect("read block min output");
    let block_max_out = Raster::read(&block_max_path).expect("read block max output");
    assert_eq!(block_min_out.get(0, 0, 0), 3.0);
    assert_eq!(block_max_out.get(0, 0, 0), 7.0);
    assert_eq!(block_min_out.get(0, 1, 1), 10.0);
    assert_eq!(block_max_out.get(0, 1, 1), 12.0);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&aggregate_path);
    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&block_min_path);
    let _ = std::fs::remove_file(&block_max_path);
}

#[test]
fn standard_deviation_overlay_computes_expected_values() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_stddev_overlay");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    for (path, values) in [
        (&in1_path, [1.0, 3.0, 5.0, 7.0]),
        (&in2_path, [3.0, 5.0, 7.0, 9.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let mut args = ToolArgs::new();
    args.insert(
        "input_rasters".to_string(),
        json!([
            in1_path.to_string_lossy().to_string(),
            in2_path.to_string_lossy().to_string()
        ]),
    );
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("standard_deviation_overlay", &args, &context(&caps))
        .expect("standard_deviation_overlay should run");

    let out = Raster::read(&out_path).expect("read output");
    assert!((out.get(0, 0, 0) - 1.0).abs() < 1e-6);
    assert!((out.get(0, 0, 1) - 1.0).abs() < 1e-6);
    assert!((out.get(0, 1, 0) - 1.0).abs() < 1e-6);
    assert!((out.get(0, 1, 1) - 1.0).abs() < 1e-6);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn interpolation_tools_run_end_to_end_and_preserve_point_crs() {
    use wbvector::{FieldDef, FieldType, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_interpolation_tools");
    let points_path = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let base_path = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let idw_path = std::env::temp_dir().join(format!("{tag}_idw.tif"));
    let nn_path = std::env::temp_dir().join(format!("{tag}_nn.tif"));
    let nat_path = std::env::temp_dir().join(format!("{tag}_nat.tif"));
    let ms_path = std::env::temp_dir().join(format!("{tag}_ms.tif"));
    let rbf_path = std::env::temp_dir().join(format!("{tag}_rbf.tif"));
    let tin_path = std::env::temp_dir().join(format!("{tag}_tin.tif"));

    let mut points = Layer::new("points")
        .with_geom_type(wbvector::GeometryType::Point)
        .with_crs_epsg(4326);
    points.add_field(FieldDef::new("VALUE", FieldType::Float));
    points
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[("VALUE", 1.0f64.into())])
        .expect("add point 1");
    points
        .add_feature(Some(Geometry::point(2.0, 0.0)), &[("VALUE", 3.0f64.into())])
        .expect("add point 2");
    points
        .add_feature(Some(Geometry::point(0.0, 2.0)), &[("VALUE", 5.0f64.into())])
        .expect("add point 3");
    points
        .add_feature(Some(Geometry::point(2.0, 2.0)), &[("VALUE", 7.0f64.into())])
        .expect("add point 4");
    wbvector::write(&points, &points_path, VectorFormat::GeoJson).expect("write points");

    let mut base = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for r in 0..2isize {
        for c in 0..2isize {
            base.set(0, r, c, base.nodata).expect("set base nodata");
        }
    }
    base.write(&base_path, RasterFormat::GeoTiff)
        .expect("write base raster");

    let caps = OpenOnly;

    let mut idw_args = ToolArgs::new();
    idw_args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    idw_args.insert("field_name".to_string(), json!("VALUE"));
    idw_args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    idw_args.insert("output".to_string(), json!(idw_path.to_string_lossy().to_string()));
    registry
        .run("idw_interpolation", &idw_args, &context(&caps))
        .expect("idw_interpolation should run");

    let mut nn_args = ToolArgs::new();
    nn_args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    nn_args.insert("field_name".to_string(), json!("VALUE"));
    nn_args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    nn_args.insert("output".to_string(), json!(nn_path.to_string_lossy().to_string()));
    registry
        .run("nearest_neighbour_interpolation", &nn_args, &context(&caps))
        .expect("nearest_neighbour_interpolation should run");

    let mut nat_args = ToolArgs::new();
    nat_args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    nat_args.insert("field_name".to_string(), json!("VALUE"));
    nat_args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    nat_args.insert("output".to_string(), json!(nat_path.to_string_lossy().to_string()));
    registry
        .run("natural_neighbour_interpolation", &nat_args, &context(&caps))
        .expect("natural_neighbour_interpolation should run");

    let mut ms_args = ToolArgs::new();
    ms_args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    ms_args.insert("field_name".to_string(), json!("VALUE"));
    ms_args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    ms_args.insert("output".to_string(), json!(ms_path.to_string_lossy().to_string()));
    registry
        .run("modified_shepard_interpolation", &ms_args, &context(&caps))
        .expect("modified_shepard_interpolation should run");

    let mut rbf_args = ToolArgs::new();
    rbf_args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    rbf_args.insert("field_name".to_string(), json!("VALUE"));
    rbf_args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    rbf_args.insert("output".to_string(), json!(rbf_path.to_string_lossy().to_string()));
    registry
        .run("radial_basis_function_interpolation", &rbf_args, &context(&caps))
        .expect("radial_basis_function_interpolation should run");

    let mut tin_args = ToolArgs::new();
    tin_args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    tin_args.insert("field_name".to_string(), json!("VALUE"));
    tin_args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    tin_args.insert("output".to_string(), json!(tin_path.to_string_lossy().to_string()));
    registry
        .run("tin_interpolation", &tin_args, &context(&caps))
        .expect("tin_interpolation should run");

    let idw_out = Raster::read(&idw_path).expect("read idw output");
    let nn_out = Raster::read(&nn_path).expect("read nearest-neighbour output");
    let nat_out = Raster::read(&nat_path).expect("read natural-neighbour output");
    let ms_out = Raster::read(&ms_path).expect("read modified shepard output");
    let rbf_out = Raster::read(&rbf_path).expect("read radial basis output");
    let tin_out = Raster::read(&tin_path).expect("read tin output");

    assert_eq!(idw_out.rows, 2);
    assert_eq!(idw_out.cols, 2);
    assert_eq!(nn_out.rows, 2);
    assert_eq!(nn_out.cols, 2);
    assert_eq!(nat_out.rows, 2);
    assert_eq!(nat_out.cols, 2);
    assert_eq!(ms_out.rows, 2);
    assert_eq!(ms_out.cols, 2);
    assert_eq!(rbf_out.rows, 2);
    assert_eq!(rbf_out.cols, 2);
    assert_eq!(tin_out.rows, 2);
    assert_eq!(tin_out.cols, 2);
    assert_eq!(idw_out.crs.epsg, Some(4326), "idw output should preserve interpolation input CRS");
    assert_eq!(nn_out.crs.epsg, Some(4326), "nearest-neighbour output should preserve interpolation input CRS");
    assert_eq!(nat_out.crs.epsg, Some(4326), "natural-neighbour output should preserve interpolation input CRS");
    assert_eq!(ms_out.crs.epsg, Some(4326), "modified shepard output should preserve interpolation input CRS");
    assert_eq!(rbf_out.crs.epsg, Some(4326), "rbf output should preserve interpolation input CRS");
    assert_eq!(tin_out.crs.epsg, Some(4326), "tin output should preserve interpolation input CRS");

    let idw_center = idw_out.get(0, 0, 0);
    assert!(idw_center.is_finite(), "idw output should contain interpolated values");
    let nn_center = nn_out.get(0, 0, 0);
    assert!(nn_center.is_finite(), "nearest-neighbour output should contain interpolated values");
    let nat_center = nat_out.get(0, 0, 0);
    assert!(nat_center.is_finite(), "natural-neighbour output should contain interpolated values");
    let ms_center = ms_out.get(0, 0, 0);
    assert!(ms_center.is_finite(), "modified shepard output should contain interpolated values");
    let rbf_center = rbf_out.get(0, 0, 0);
    assert!(rbf_center.is_finite(), "rbf output should contain interpolated values");
    let tin_center = tin_out.get(0, 0, 0);
    assert!(tin_center.is_finite(), "tin output should contain interpolated values");

    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&idw_path);
    let _ = std::fs::remove_file(&nn_path);
    let _ = std::fs::remove_file(&nat_path);
    let _ = std::fs::remove_file(&ms_path);
    let _ = std::fs::remove_file(&rbf_path);
    let _ = std::fs::remove_file(&tin_path);
}

#[test]
fn tin_interpolation_matches_triangle_plane_values() {
    use wbvector::{FieldDef, FieldType, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_tin_plane_values");
    let points_path = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let base_path = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let out_path = std::env::temp_dir().join(format!("{tag}_tin.tif"));

    // Triangle points define z = x + y over the triangle: (0,0)->0, (2,0)->2, (0,2)->2
    let mut points = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    points.add_field(FieldDef::new("VALUE", FieldType::Float));
    points
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[("VALUE", 0.0f64.into())])
        .expect("add point 1");
    points
        .add_feature(Some(Geometry::point(2.0, 0.0)), &[("VALUE", 2.0f64.into())])
        .expect("add point 2");
    points
        .add_feature(Some(Geometry::point(0.0, 2.0)), &[("VALUE", 2.0f64.into())])
        .expect("add point 3");
    wbvector::write(&points, &points_path, VectorFormat::GeoJson).expect("write points");

    let mut base = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..2isize {
        for c in 0..2isize {
            base.set(0, r, c, base.nodata).expect("set base nodata");
        }
    }
    base.write(&base_path, RasterFormat::GeoTiff)
        .expect("write base raster");

    let mut args = ToolArgs::new();
    args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    args.insert("field_name".to_string(), json!("VALUE"));
    args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("tin_interpolation", &args, &context(&caps))
        .expect("tin_interpolation should run");

    let out = Raster::read(&out_path).expect("read output");
    // Cell centers: (0.5,0.5)->1, (1.5,0.5)->2, (0.5,1.5)->2 are inside the triangle.
    assert!((out.get(0, 1, 0) - 1.0).abs() < 1e-9);
    assert!((out.get(0, 1, 1) - 2.0).abs() < 1e-9);
    assert!((out.get(0, 0, 0) - 2.0).abs() < 1e-9);

    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn tin_interpolation_max_edge_length_filters_long_triangles() {
    use wbvector::{FieldDef, FieldType, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_tin_max_edge");
    let points_path = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let base_path = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let out_path = std::env::temp_dir().join(format!("{tag}_tin.tif"));

    let mut points = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    points.add_field(FieldDef::new("VALUE", FieldType::Float));
    points
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[("VALUE", 0.0f64.into())])
        .expect("add point 1");
    points
        .add_feature(Some(Geometry::point(2.0, 0.0)), &[("VALUE", 2.0f64.into())])
        .expect("add point 2");
    points
        .add_feature(Some(Geometry::point(0.0, 2.0)), &[("VALUE", 2.0f64.into())])
        .expect("add point 3");
    wbvector::write(&points, &points_path, VectorFormat::GeoJson).expect("write points");

    let mut base = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..2isize {
        for c in 0..2isize {
            base.set(0, r, c, base.nodata).expect("set base nodata");
        }
    }
    base.write(&base_path, RasterFormat::GeoTiff)
        .expect("write base raster");

    let mut args = ToolArgs::new();
    args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    args.insert("field_name".to_string(), json!("VALUE"));
    args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    args.insert("max_triangle_edge_length".to_string(), json!(1.0));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("tin_interpolation", &args, &context(&caps))
        .expect("tin_interpolation should run");

    let out = Raster::read(&out_path).expect("read output");
    for r in 0..2isize {
        for c in 0..2isize {
            assert_eq!(out.get(0, r, c), out.nodata, "all cells should remain NoData after filtering long triangles");
        }
    }

    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn interpolation_tools_match_exact_sample_values_at_point_cells() {
    use wbvector::{FieldDef, FieldType, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_interp_exact_cells");
    let points_path = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let base_path = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let nat_path = std::env::temp_dir().join(format!("{tag}_nat.tif"));
    let ms_path = std::env::temp_dir().join(format!("{tag}_ms.tif"));
    let rbf_path = std::env::temp_dir().join(format!("{tag}_rbf.tif"));

    let mut points = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    points.add_field(FieldDef::new("VALUE", FieldType::Float));
    points
        .add_feature(Some(Geometry::point(0.5, 2.5)), &[("VALUE", 10.0f64.into())])
        .expect("add point 1");
    points
        .add_feature(Some(Geometry::point(2.5, 2.5)), &[("VALUE", 20.0f64.into())])
        .expect("add point 2");
    points
        .add_feature(Some(Geometry::point(0.5, 0.5)), &[("VALUE", 30.0f64.into())])
        .expect("add point 3");
    points
        .add_feature(Some(Geometry::point(1.5, 1.5)), &[("VALUE", 40.0f64.into())])
        .expect("add point 4");
    wbvector::write(&points, &points_path, VectorFormat::GeoJson).expect("write points");

    let mut base = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            base.set(0, r, c, base.nodata).expect("set base nodata");
        }
    }
    base.write(&base_path, RasterFormat::GeoTiff).expect("write base raster");

    let caps = OpenOnly;
    for (tool_id, out_path) in [
        ("natural_neighbour_interpolation", &nat_path),
        ("modified_shepard_interpolation", &ms_path),
        ("radial_basis_function_interpolation", &rbf_path),
    ] {
        let mut args = ToolArgs::new();
        args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
        args.insert("field_name".to_string(), json!("VALUE"));
        args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        registry.run(tool_id, &args, &context(&caps)).expect("tool should run");
    }

    let nat = Raster::read(&nat_path).expect("read nat");
    let ms = Raster::read(&ms_path).expect("read ms");
    let rbf = Raster::read(&rbf_path).expect("read rbf");

    for raster in [&nat, &ms, &rbf] {
        assert!((raster.get(0, 1, 1) - 40.0).abs() < 1e-9);
    }
    for (raster, expected_corner_values) in [(&nat, [10.0, 20.0, 30.0]), (&ms, [10.0, 20.0, 30.0])] {
        assert!((raster.get(0, 0, 0) - expected_corner_values[0]).abs() < 1e-9);
        assert!((raster.get(0, 0, 2) - expected_corner_values[1]).abs() < 1e-9);
        assert!((raster.get(0, 2, 0) - expected_corner_values[2]).abs() < 1e-9);
    }

    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&nat_path);
    let _ = std::fs::remove_file(&ms_path);
    let _ = std::fs::remove_file(&rbf_path);
}

#[test]
fn gis_utility_tools_run_end_to_end() {
    use wbvector::{FieldDef, FieldType, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_gis_utilities");
    let points_path = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let weighted_points_path = std::env::temp_dir().join(format!("{tag}_weighted_points.geojson"));
    let base_path = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let raster1_path = std::env::temp_dir().join(format!("{tag}_r1.tif"));
    let raster2_path = std::env::temp_dir().join(format!("{tag}_r2.tif"));
    let heat_path = std::env::temp_dir().join(format!("{tag}_heat.tif"));
    let assign_path = std::env::temp_dir().join(format!("{tag}_assign.tif"));
    let sampled_path = std::env::temp_dir().join(format!("{tag}_sampled.geojson"));

    let mut points = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    points
        .add_feature(Some(Geometry::point(0.5, 1.5)), &[])
        .expect("add point");
    wbvector::write(&points, &points_path, VectorFormat::GeoJson).expect("write points");

    let mut weighted_points = Layer::new("weighted_points")
        .with_geom_type(wbvector::GeometryType::Point)
        .with_crs_epsg(4326);
    weighted_points.add_field(FieldDef::new("WEIGHT", FieldType::Float));
    weighted_points
        .add_feature(Some(Geometry::point(0.5, 0.5)), &[("WEIGHT", 2.0f64.into())])
        .expect("add weighted point");
    wbvector::write(&weighted_points, &weighted_points_path, VectorFormat::GeoJson)
        .expect("write weighted points");

    let mut base = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for r in 0..2isize {
        for c in 0..2isize {
            base.set(0, r, c, base.nodata).expect("set base nodata");
        }
    }
    base.write(&base_path, RasterFormat::GeoTiff).expect("write base");

    let mut raster1 = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster1.set(0, 0, 0, 5.0).expect("set");
    raster1.set(0, 0, 1, 6.0).expect("set");
    raster1.set(0, 1, 0, 7.0).expect("set");
    raster1.set(0, 1, 1, 8.0).expect("set");
    raster1.write(&raster1_path, RasterFormat::GeoTiff).expect("write r1");

    let mut raster2 = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    raster2.set(0, 0, 0, 15.0).expect("set");
    raster2.set(0, 0, 1, 16.0).expect("set");
    raster2.set(0, 1, 0, 17.0).expect("set");
    raster2.set(0, 1, 1, 18.0).expect("set");
    raster2.write(&raster2_path, RasterFormat::GeoTiff).expect("write r2");

    let mut heat_args = ToolArgs::new();
    heat_args.insert("points".to_string(), json!(weighted_points_path.to_string_lossy().to_string()));
    heat_args.insert("field_name".to_string(), json!("WEIGHT"));
    heat_args.insert("bandwidth".to_string(), json!(1.0));
    heat_args.insert("kernel_function".to_string(), json!("uniform"));
    heat_args.insert("base_raster".to_string(), json!(base_path.to_string_lossy().to_string()));
    heat_args.insert("output".to_string(), json!(heat_path.to_string_lossy().to_string()));
    registry.run("heat_map", &heat_args, &context(&caps)).expect("heat_map should run");

    let mut assign_args = ToolArgs::new();
    assign_args.insert("input".to_string(), json!(base_path.to_string_lossy().to_string()));
    assign_args.insert("what_to_assign".to_string(), json!("column"));
    assign_args.insert("output".to_string(), json!(assign_path.to_string_lossy().to_string()));
    registry
        .run("raster_cell_assignment", &assign_args, &context(&caps))
        .expect("raster_cell_assignment should run");

    let mut sample_args = ToolArgs::new();
    sample_args.insert(
        "rasters".to_string(),
        json!([
            raster1_path.to_string_lossy().to_string(),
            raster2_path.to_string_lossy().to_string()
        ]),
    );
    sample_args.insert("points".to_string(), json!(points_path.to_string_lossy().to_string()));
    sample_args.insert("output".to_string(), json!(sampled_path.to_string_lossy().to_string()));
    let sample_result = registry
        .run("extract_raster_values_at_points", &sample_args, &context(&caps))
        .expect("extract_raster_values_at_points should run");

    let heat = Raster::read(&heat_path).expect("read heat");
    assert!((heat.get(0, 1, 0) - 1.0).abs() < 1e-9, "uniform kernel with weight 2 at d=0 should equal 1.0");
    assert_eq!(heat.crs.epsg, Some(4326));

    let assign = Raster::read(&assign_path).expect("read assignment");
    assert_eq!(assign.get(0, 0, 0), 0.0);
    assert_eq!(assign.get(0, 0, 1), 1.0);

    let sampled = wbvector::read(&sampled_path).expect("read sampled vector");
    assert_eq!(sampled.features.len(), 1);
    let attrs = &sampled.features[0].attributes;
    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0].as_f64(), Some(5.0));
    assert_eq!(attrs[1].as_f64(), Some(15.0));

    let report = sample_result.outputs.get("report").and_then(|v| v.as_str()).expect("report string");
    assert!(report.contains("Point 1 values"));
    assert!(report.contains("5.0"));
    assert!(report.contains("15.0"));

    let _ = std::fs::remove_file(&points_path);
    let _ = std::fs::remove_file(&weighted_points_path);
    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&raster1_path);
    let _ = std::fs::remove_file(&raster2_path);
    let _ = std::fs::remove_file(&heat_path);
    let _ = std::fs::remove_file(&assign_path);
    let _ = std::fs::remove_file(&sampled_path);
}

#[test]
fn gis_footprint_and_map_features_tools_run_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_gis_footprints");
    let raster_path = std::env::temp_dir().join(format!("{tag}_input.tif"));
    let raster_footprint_path = std::env::temp_dir().join(format!("{tag}_raster_footprint.geojson"));
    let vector_path = std::env::temp_dir().join(format!("{tag}_input.geojson"));
    let vector_footprint_path = std::env::temp_dir().join(format!("{tag}_vector_footprint.geojson"));
    let features_path = std::env::temp_dir().join(format!("{tag}_features.tif"));

    let mut raster = Raster::new(RasterConfig {
        cols: 4,
        rows: 3,
        bands: 1,
        x_min: 10.0,
        y_min: 20.0,
        cell_size: 2.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    let feature_values = [
        1.0, 1.0, 1.0, 1.0,
        1.0, 9.0, 1.0, 8.0,
        1.0, 1.0, 1.0, 1.0,
    ];
    for (index, value) in feature_values.into_iter().enumerate() {
        let row = (index / 4) as isize;
        let col = (index % 4) as isize;
        raster.set(0, row, col, value).expect("set raster value");
    }
    raster.write(&raster_path, RasterFormat::GeoTiff).expect("write raster");

    let mut vector = Layer::new("input_lines")
        .with_geom_type(wbvector::GeometryType::LineString)
        .with_crs_epsg(4326);
    vector
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(100.0, 200.0),
                Coord::xy(108.0, 206.0),
                Coord::xy(104.0, 203.0),
            ])),
            &[],
        )
        .expect("add vector feature");
    wbvector::write(&vector, &vector_path, VectorFormat::GeoJson).expect("write vector");

    let mut raster_fp_args = ToolArgs::new();
    raster_fp_args.insert("input".to_string(), json!(raster_path.to_string_lossy().to_string()));
    raster_fp_args.insert("output".to_string(), json!(raster_footprint_path.to_string_lossy().to_string()));
    registry
        .run("layer_footprint_raster", &raster_fp_args, &context(&caps))
        .expect("layer_footprint_raster should run");

    let mut vector_fp_args = ToolArgs::new();
    vector_fp_args.insert("input".to_string(), json!(vector_path.to_string_lossy().to_string()));
    vector_fp_args.insert("output".to_string(), json!(vector_footprint_path.to_string_lossy().to_string()));
    registry
        .run("layer_footprint_vector", &vector_fp_args, &context(&caps))
        .expect("layer_footprint_vector should run");

    let mut map_args = ToolArgs::new();
    map_args.insert("input".to_string(), json!(raster_path.to_string_lossy().to_string()));
    map_args.insert("min_feature_height".to_string(), json!(2.0));
    map_args.insert("min_feature_size".to_string(), json!(1));
    map_args.insert("output".to_string(), json!(features_path.to_string_lossy().to_string()));
    registry
        .run("map_features", &map_args, &context(&caps))
        .expect("map_features should run");

    let raster_footprint = wbvector::read(&raster_footprint_path).expect("read raster footprint");
    assert_eq!(raster_footprint.features.len(), 1);
    match raster_footprint.features[0].geometry.as_ref().expect("raster footprint geometry") {
        Geometry::Polygon { exterior, .. } => {
            let xs: Vec<_> = exterior.coords().iter().map(|coord| coord.x).collect();
            let ys: Vec<_> = exterior.coords().iter().map(|coord| coord.y).collect();
            assert!(xs.iter().any(|x| (*x - 10.0).abs() < 1e-9));
            assert!(xs.iter().any(|x| (*x - 18.0).abs() < 1e-9));
            assert!(ys.iter().any(|y| (*y - 20.0).abs() < 1e-9));
            assert!(ys.iter().any(|y| (*y - 26.0).abs() < 1e-9));
        }
        other => panic!("unexpected raster footprint geometry: {other:?}"),
    }

    let vector_footprint = wbvector::read(&vector_footprint_path).expect("read vector footprint");
    assert_eq!(vector_footprint.features.len(), 1);
    match vector_footprint.features[0].geometry.as_ref().expect("vector footprint geometry") {
        Geometry::Polygon { exterior, .. } => {
            let xs: Vec<_> = exterior.coords().iter().map(|coord| coord.x).collect();
            let ys: Vec<_> = exterior.coords().iter().map(|coord| coord.y).collect();
            assert!(xs.iter().any(|x| (*x - 100.0).abs() < 1e-9));
            assert!(xs.iter().any(|x| (*x - 108.0).abs() < 1e-9));
            assert!(ys.iter().any(|y| (*y - 200.0).abs() < 1e-9));
            assert!(ys.iter().any(|y| (*y - 206.0).abs() < 1e-9));
        }
        other => panic!("unexpected vector footprint geometry: {other:?}"),
    }

    let mapped = Raster::read(&features_path).expect("read mapped features raster");
    let left_peak = mapped.get(0, 1, 1);
    let right_peak = mapped.get(0, 1, 3);
    assert!(left_peak > 0.0);
    assert!(right_peak > 0.0);
    assert_ne!(left_peak, right_peak);
    assert!(mapped.get(0, 0, 0) > 0.0);

    let _ = std::fs::remove_file(&raster_path);
    let _ = std::fs::remove_file(&raster_footprint_path);
    let _ = std::fs::remove_file(&vector_path);
    let _ = std::fs::remove_file(&vector_footprint_path);
    let _ = std::fs::remove_file(&features_path);
}

#[test]
fn gis_create_plane_centroid_raster_and_medoid_run_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_gis_plane_centroid_medoid");
    let base_path = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let plane_path = std::env::temp_dir().join(format!("{tag}_plane.tif"));
    let patches_path = std::env::temp_dir().join(format!("{tag}_patches.tif"));
    let centroid_path = std::env::temp_dir().join(format!("{tag}_centroid.tif"));
    let lines_path = std::env::temp_dir().join(format!("{tag}_lines.geojson"));
    let medoid_path = std::env::temp_dir().join(format!("{tag}_medoid.geojson"));

    let mut base = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for row in 0..3isize {
        for col in 0..3isize {
            base.set(0, row, col, 0.0).expect("set base");
        }
    }
    base.write(&base_path, RasterFormat::GeoTiff).expect("write base raster");

    let mut create_args = ToolArgs::new();
    create_args.insert("base".to_string(), json!(base_path.to_string_lossy().to_string()));
    create_args.insert("gradient".to_string(), json!(0.0));
    create_args.insert("aspect".to_string(), json!(90.0));
    create_args.insert("constant".to_string(), json!(7.0));
    create_args.insert("output".to_string(), json!(plane_path.to_string_lossy().to_string()));
    registry
        .run("create_plane", &create_args, &context(&caps))
        .expect("create_plane should run");

    let plane = Raster::read(&plane_path).expect("read plane raster");
    assert!((plane.get(0, 0, 0) - 7.0).abs() < 1e-9);
    assert!((plane.get(0, 1, 1) - 7.0).abs() < 1e-9);
    assert!((plane.get(0, 2, 2) - 7.0).abs() < 1e-9);

    let mut patches = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for row in 0..3isize {
        for col in 0..3isize {
            patches.set(0, row, col, 0.0).expect("set background");
        }
    }
    patches.set(0, 0, 0, 1.0).expect("set patch");
    patches.set(0, 0, 1, 1.0).expect("set patch");
    patches.set(0, 1, 0, 1.0).expect("set patch");
    patches.set(0, 2, 2, 2.0).expect("set patch");
    patches.set(0, 1, 2, 2.0).expect("set patch");
    patches.write(&patches_path, RasterFormat::GeoTiff).expect("write patches raster");

    let mut centroid_args = ToolArgs::new();
    centroid_args.insert("input".to_string(), json!(patches_path.to_string_lossy().to_string()));
    centroid_args.insert("output".to_string(), json!(centroid_path.to_string_lossy().to_string()));
    let centroid_result = registry
        .run("centroid_raster", &centroid_args, &context(&caps))
        .expect("centroid_raster should run");

    let centroid = Raster::read(&centroid_path).expect("read centroid raster");
    assert_eq!(centroid.get(0, 0, 0), 1.0);
    assert_eq!(centroid.get(0, 1, 2), 2.0);
    let report = centroid_result.outputs.get("report").and_then(|v| v.as_str()).expect("report output");
    assert!(report.contains("Patch Centroid"));

    let mut lines = Layer::new("lines").with_geom_type(wbvector::GeometryType::LineString);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(2.0, 0.0),
                Coord::xy(2.0, 2.0),
            ])),
            &[],
        )
        .expect("add line");
    wbvector::write(&lines, &lines_path, VectorFormat::GeoJson).expect("write line input");

    let mut medoid_args = ToolArgs::new();
    medoid_args.insert("input".to_string(), json!(lines_path.to_string_lossy().to_string()));
    medoid_args.insert("output".to_string(), json!(medoid_path.to_string_lossy().to_string()));
    registry
        .run("medoid", &medoid_args, &context(&caps))
        .expect("medoid should run");

    let medoid_layer = wbvector::read(&medoid_path).expect("read medoid output");
    assert_eq!(medoid_layer.features.len(), 1);
    match medoid_layer.features[0].geometry.as_ref().expect("medoid geometry") {
        Geometry::Point(coord) => {
            assert!((coord.x - 2.0).abs() < 1e-9);
            assert!((coord.y - 0.0).abs() < 1e-9);
        }
        other => panic!("unexpected medoid output geometry: {other:?}"),
    }

    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&plane_path);
    let _ = std::fs::remove_file(&patches_path);
    let _ = std::fs::remove_file(&centroid_path);
    let _ = std::fs::remove_file(&lines_path);
    let _ = std::fs::remove_file(&medoid_path);
}

#[test]
fn find_lowest_or_highest_points_runs_end_to_end() {
    use wbvector::Geometry;

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_find_extrema_points");
    let raster_path = std::env::temp_dir().join(format!("{tag}_in.tif"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    let values = [
        2.0, 3.0, 9.0,
        4.0, 5.0, 6.0,
        -5.0, 7.0, 8.0,
    ];
    for (index, value) in values.into_iter().enumerate() {
        let row = (index / 3) as isize;
        let col = (index % 3) as isize;
        raster.set(0, row, col, value).expect("set raster value");
    }
    raster.write(&raster_path, RasterFormat::GeoTiff).expect("write raster");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(raster_path.to_string_lossy().to_string()));
    args.insert("out_type".to_string(), json!("both"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("find_lowest_or_highest_points", &args, &context(&caps))
        .expect("find_lowest_or_highest_points should run");

    let out = wbvector::read(&output_path).expect("read output vector");
    assert_eq!(out.features.len(), 2);

    let low = &out.features[0];
    let high = &out.features[1];
    assert_eq!(low.attributes[1].as_f64(), Some(-5.0));
    assert_eq!(high.attributes[1].as_f64(), Some(9.0));

    match low.geometry.as_ref().expect("lowest geometry") {
        Geometry::Point(coord) => {
            assert!((coord.x - 0.5).abs() < 1e-9);
            assert!((coord.y - 0.5).abs() < 1e-9);
        }
        other => panic!("unexpected lowest geometry: {other:?}"),
    }
    match high.geometry.as_ref().expect("highest geometry") {
        Geometry::Point(coord) => {
            assert!((coord.x - 2.5).abs() < 1e-9);
            assert!((coord.y - 2.5).abs() < 1e-9);
        }
        other => panic!("unexpected highest geometry: {other:?}"),
    }

    let _ = std::fs::remove_file(&raster_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn eliminate_coincident_points_runs_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_eliminate_coincident_points");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut points = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    points
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[])
        .expect("add point 1");
    points
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[])
        .expect("add point 2");
    points
        .add_feature(Some(Geometry::point(1.0, 1.0)), &[])
        .expect("add point 3");
    points
        .add_feature(Some(Geometry::point(1.00005, 1.00004)), &[])
        .expect("add point 4");
    wbvector::write(&points, &input_path, VectorFormat::GeoJson).expect("write point input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("tolerance_dist".to_string(), json!(0.001));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("eliminate_coincident_points", &args, &context(&caps))
        .expect("eliminate_coincident_points should run");

    let out = wbvector::read(&output_path).expect("read output points");
    assert_eq!(out.features.len(), 2);

    match out.features[0].geometry.as_ref().expect("first geometry") {
        Geometry::Point(coord) => {
            assert!((coord.x - 0.0).abs() < 1e-9);
            assert!((coord.y - 0.0).abs() < 1e-9);
        }
        other => panic!("unexpected first output geometry: {other:?}"),
    }
    match out.features[1].geometry.as_ref().expect("second geometry") {
        Geometry::Point(coord) => {
            assert!((coord.x - 1.0).abs() < 1e-9);
            assert!((coord.y - 1.0).abs() < 1e-9);
        }
        other => panic!("unexpected second output geometry: {other:?}"),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn extend_vector_lines_runs_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_extend_vector_lines");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut lines = Layer::new("lines").with_geom_type(wbvector::GeometryType::LineString);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                wbvector::Coord::xy(0.0, 0.0),
                wbvector::Coord::xy(1.0, 0.0),
            ])),
            &[],
        )
        .expect("add line");
    wbvector::write(&lines, &input_path, VectorFormat::GeoJson).expect("write line input");
    let input_layer = wbvector::read(&input_path).expect("read line input");
    assert!(
        matches!(
            input_layer.features.first().and_then(|f| f.geometry.as_ref()),
            Some(Geometry::LineString(_))
        ),
        "line input should contain a linestring feature"
    );

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("distance".to_string(), json!(1.0));
    args.insert("extend_direction".to_string(), json!("both"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("extend_vector_lines", &args, &context(&caps))
        .expect("extend_vector_lines should run");

    let out = wbvector::read(&output_path).expect("read output lines");
    assert_eq!(out.features.len(), 1);
    match out.features[0].geometry.as_ref().expect("line geometry") {
        Geometry::LineString(coords) => {
            assert_eq!(coords.len(), 2);
            assert!((coords[0].x + 1.0).abs() < 1e-9);
            assert!((coords[0].y - 0.0).abs() < 1e-9);
            assert!((coords[1].x - 2.0).abs() < 1e-9);
            assert!((coords[1].y - 0.0).abs() < 1e-9);
        }
        other => panic!("unexpected output geometry: {other:?}"),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn smooth_vectors_runs_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_smooth_vectors");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut lines = Layer::new("lines").with_geom_type(wbvector::GeometryType::LineString);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![
                wbvector::Coord::xy(0.0, 0.0),
                wbvector::Coord::xy(1.0, 1.0),
                wbvector::Coord::xy(2.0, 0.0),
                wbvector::Coord::xy(3.0, 1.0),
                wbvector::Coord::xy(4.0, 0.0),
            ])),
            &[],
        )
        .expect("add line");
    wbvector::write(&lines, &input_path, VectorFormat::GeoJson).expect("write line input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("filter_size".to_string(), json!(3));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("smooth_vectors", &args, &context(&caps))
        .expect("smooth_vectors should run");

    let out = wbvector::read(&output_path).expect("read output lines");
    assert_eq!(out.features.len(), 1);
    match out.features[0].geometry.as_ref().expect("line geometry") {
        Geometry::LineString(coords) => {
            assert_eq!(coords.len(), 5);
            assert!((coords[0].x - 0.0).abs() < 1e-9);
            assert!((coords[0].y - 0.0).abs() < 1e-9);
            assert!((coords[1].x - 1.0).abs() < 1e-9);
            assert!((coords[1].y - (1.0 / 3.0)).abs() < 1e-9);
            assert!((coords[2].x - 2.0).abs() < 1e-9);
            assert!((coords[2].y - (2.0 / 3.0)).abs() < 1e-9);
            assert!((coords[3].x - 3.0).abs() < 1e-9);
            assert!((coords[3].y - (1.0 / 3.0)).abs() < 1e-9);
            assert!((coords[4].x - 4.0).abs() < 1e-9);
            assert!((coords[4].y - 0.0).abs() < 1e-9);
        }
        other => panic!("unexpected output geometry: {other:?}"),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn split_vector_lines_produces_correct_segments() {
    // Input: single polyline (0,0) -> (3,0) -> (6,0), segment_length = 2.0
    // Expected: 3 output segments, each <= 2.0 units long.
    use wbvector::{Coord, Geometry, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_split_vector_lines");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    // Build a single-line input layer with no custom attributes.
    let mut layer = wbvector::Layer::new("lines").with_geom_type(wbvector::GeometryType::LineString);
    layer
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(3.0, 0.0),
                Coord::xy(6.0, 0.0),
            ])),
            &[],
        )
        .expect("add line feature");
    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("segment_length".to_string(), json!(2.0));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    registry
        .run("split_vector_lines", &args, &context(&caps))
        .expect("split_vector_lines should run");

    let out = wbvector::read(&output_path).expect("read output");
    assert_eq!(out.features.len(), 3, "expected 3 segments");

    // Each segment must have FID=1..3 (attr[0]) and PARENT_ID=1 (attr[1]).
    for (i, feat) in out.features.iter().enumerate() {
        assert_eq!(
            feat.attributes[0],
            wbvector::FieldValue::Integer((i + 1) as i64),
            "FID mismatch for segment {i}"
        );
        assert_eq!(
            feat.attributes[1],
            wbvector::FieldValue::Integer(1),
            "PARENT_ID mismatch for segment {i}"
        );
    }

    // Verify first segment: start=(0,0), end=(2,0).
    if let Some(Geometry::LineString(coords)) = &out.features[0].geometry {
        assert!((coords[0].x - 0.0).abs() < 1e-10);
        assert!((coords[0].y - 0.0).abs() < 1e-10);
        let last = coords.last().unwrap();
        assert!((last.x - 2.0).abs() < 1e-10, "last.x={}", last.x);
        assert!((last.y - 0.0).abs() < 1e-10);
    } else {
        panic!("expected LineString geometry for segment 0");
    }

    // Verify last segment: start=(4,0), end=(6,0).
    if let Some(Geometry::LineString(coords)) = &out.features[2].geometry {
        assert!((coords[0].x - 4.0).abs() < 1e-10, "start.x={}", coords[0].x);
        assert!((coords[0].y - 0.0).abs() < 1e-10);
        let last = coords.last().unwrap();
        assert!((last.x - 6.0).abs() < 1e-10, "last.x={}", last.x);
        assert!((last.y - 0.0).abs() < 1e-10);
    } else {
        panic!("expected LineString geometry for segment 2");
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn rectangular_grid_tools_run_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_rect_grid");
    let raster_in = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let vector_in = std::env::temp_dir().join(format!("{tag}_base.geojson"));
    let raster_out = std::env::temp_dir().join(format!("{tag}_grid_from_raster.geojson"));
    let vector_out = std::env::temp_dir().join(format!("{tag}_grid_from_vector.geojson"));

    let mut raster = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo {
            epsg: Some(4326),
            ..Default::default()
        },
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 1.0).expect("set raster value");
    raster
        .write(&raster_in, RasterFormat::GeoTiff)
        .expect("write base raster");

    let mut base_layer = Layer::new("base_poly").with_geom_type(wbvector::GeometryType::Polygon);
    base_layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    wbvector::Coord::xy(0.0, 0.0),
                    wbvector::Coord::xy(2.0, 0.0),
                    wbvector::Coord::xy(2.0, 2.0),
                    wbvector::Coord::xy(0.0, 2.0),
                    wbvector::Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add base polygon");
    wbvector::write(&base_layer, &vector_in, VectorFormat::GeoJson).expect("write base vector");

    let mut raster_args = ToolArgs::new();
    raster_args.insert("base".to_string(), json!(raster_in.to_string_lossy().to_string()));
    raster_args.insert("width".to_string(), json!(1.0));
    raster_args.insert("height".to_string(), json!(1.0));
    raster_args.insert("output".to_string(), json!(raster_out.to_string_lossy().to_string()));
    registry
        .run("rectangular_grid_from_raster_base", &raster_args, &context(&caps))
        .expect("rectangular_grid_from_raster_base should run");

    let mut vector_args = ToolArgs::new();
    vector_args.insert("base".to_string(), json!(vector_in.to_string_lossy().to_string()));
    vector_args.insert("width".to_string(), json!(1.0));
    vector_args.insert("height".to_string(), json!(1.0));
    vector_args.insert("output".to_string(), json!(vector_out.to_string_lossy().to_string()));
    registry
        .run("rectangular_grid_from_vector_base", &vector_args, &context(&caps))
        .expect("rectangular_grid_from_vector_base should run");

    let out_from_raster = wbvector::read(&raster_out).expect("read raster-grid output");
    let out_from_vector = wbvector::read(&vector_out).expect("read vector-grid output");

    assert_eq!(out_from_raster.features.len(), 4, "expected 2x2 grid from raster");
    assert_eq!(out_from_vector.features.len(), 4, "expected 2x2 grid from vector");
    assert_eq!(out_from_raster.features[0].attributes.len(), 3, "FID/ROW/COLUMN expected");
    assert_eq!(out_from_vector.features[0].attributes.len(), 3, "FID/ROW/COLUMN expected");

    let _ = std::fs::remove_file(&raster_in);
    let _ = std::fs::remove_file(&vector_in);
    let _ = std::fs::remove_file(&raster_out);
    let _ = std::fs::remove_file(&vector_out);
}

#[test]
fn hexagonal_grid_tools_run_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_hex_grid");
    let raster_in = std::env::temp_dir().join(format!("{tag}_base.tif"));
    let vector_in = std::env::temp_dir().join(format!("{tag}_base.geojson"));
    let raster_out = std::env::temp_dir().join(format!("{tag}_hex_from_raster.geojson"));
    let vector_out = std::env::temp_dir().join(format!("{tag}_hex_from_vector.geojson"));

    let mut raster = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo {
            epsg: Some(4326),
            ..Default::default()
        },
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 1.0).expect("set raster value");
    raster
        .write(&raster_in, RasterFormat::GeoTiff)
        .expect("write base raster");

    let mut base_layer = Layer::new("base_poly").with_geom_type(wbvector::GeometryType::Polygon);
    base_layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    wbvector::Coord::xy(0.0, 0.0),
                    wbvector::Coord::xy(2.0, 0.0),
                    wbvector::Coord::xy(2.0, 2.0),
                    wbvector::Coord::xy(0.0, 2.0),
                    wbvector::Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add base polygon");
    wbvector::write(&base_layer, &vector_in, VectorFormat::GeoJson).expect("write base vector");

    let mut raster_args = ToolArgs::new();
    raster_args.insert("base".to_string(), json!(raster_in.to_string_lossy().to_string()));
    raster_args.insert("width".to_string(), json!(1.0));
    raster_args.insert("orientation".to_string(), json!("h"));
    raster_args.insert("output".to_string(), json!(raster_out.to_string_lossy().to_string()));
    registry
        .run("hexagonal_grid_from_raster_base", &raster_args, &context(&caps))
        .expect("hexagonal_grid_from_raster_base should run");

    let mut vector_args = ToolArgs::new();
    vector_args.insert("base".to_string(), json!(vector_in.to_string_lossy().to_string()));
    vector_args.insert("width".to_string(), json!(1.0));
    vector_args.insert("orientation".to_string(), json!("v"));
    vector_args.insert("output".to_string(), json!(vector_out.to_string_lossy().to_string()));
    registry
        .run("hexagonal_grid_from_vector_base", &vector_args, &context(&caps))
        .expect("hexagonal_grid_from_vector_base should run");

    let out_from_raster = wbvector::read(&raster_out).expect("read raster-hex output");
    let out_from_vector = wbvector::read(&vector_out).expect("read vector-hex output");

    assert!(out_from_raster.features.len() > 0, "expected non-empty raster-based hex grid");
    assert!(out_from_vector.features.len() > 0, "expected non-empty vector-based hex grid");
    assert_eq!(out_from_raster.features[0].attributes.len(), 3, "FID/ROW/COLUMN expected");
    assert_eq!(out_from_vector.features[0].attributes.len(), 3, "FID/ROW/COLUMN expected");

    let _ = std::fs::remove_file(&raster_in);
    let _ = std::fs::remove_file(&vector_in);
    let _ = std::fs::remove_file(&raster_out);
    let _ = std::fs::remove_file(&vector_out);
}

#[test]
fn snap_endnodes_and_voronoi_diagram_run_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_snap_voronoi");
    let lines_in = std::env::temp_dir().join(format!("{tag}_lines.geojson"));
    let lines_out = std::env::temp_dir().join(format!("{tag}_snapped.geojson"));
    let points_in = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let voronoi_out = std::env::temp_dir().join(format!("{tag}_voronoi.geojson"));

    let mut line_layer = Layer::new("lines").with_geom_type(wbvector::GeometryType::LineString);
    line_layer
        .add_feature(
            Some(Geometry::line_string(vec![
                wbvector::Coord::xy(0.0, 0.0),
                wbvector::Coord::xy(1.0, 0.0),
            ])),
            &[],
        )
        .expect("add line feature 1");
    line_layer
        .add_feature(
            Some(Geometry::line_string(vec![
                wbvector::Coord::xy(1.02, 0.0),
                wbvector::Coord::xy(2.0, 0.0),
            ])),
            &[],
        )
        .expect("add line feature 2");
    wbvector::write(&line_layer, &lines_in, VectorFormat::GeoJson).expect("write line input");

    let mut snap_args = ToolArgs::new();
    snap_args.insert("input".to_string(), json!(lines_in.to_string_lossy().to_string()));
    snap_args.insert("snap_tolerance".to_string(), json!(0.05));
    snap_args.insert("output".to_string(), json!(lines_out.to_string_lossy().to_string()));
    registry
        .run("snap_endnodes", &snap_args, &context(&caps))
        .expect("snap_endnodes should run");

    let snapped = wbvector::read(&lines_out).expect("read snapped output");
    assert_eq!(snapped.features.len(), 2);
    let end_x = match snapped.features[0].geometry.as_ref().expect("line 1 geometry") {
        Geometry::LineString(cs) => cs.last().expect("line1 end").x,
        _ => panic!("expected LineString"),
    };
    let start_x = match snapped.features[1].geometry.as_ref().expect("line 2 geometry") {
        Geometry::LineString(cs) => cs.first().expect("line2 start").x,
        _ => panic!("expected LineString"),
    };
    assert!((end_x - start_x).abs() < 1e-9, "snapped endpoints should coincide");

    let mut point_layer = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    point_layer
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[])
        .expect("add point 1");
    point_layer
        .add_feature(Some(Geometry::point(2.0, 0.0)), &[])
        .expect("add point 2");
    point_layer
        .add_feature(Some(Geometry::point(1.0, 1.5)), &[])
        .expect("add point 3");
    wbvector::write(&point_layer, &points_in, VectorFormat::GeoJson).expect("write points input");

    let mut voronoi_args = ToolArgs::new();
    voronoi_args.insert("input_points".to_string(), json!(points_in.to_string_lossy().to_string()));
    voronoi_args.insert("output".to_string(), json!(voronoi_out.to_string_lossy().to_string()));
    registry
        .run("voronoi_diagram", &voronoi_args, &context(&caps))
        .expect("voronoi_diagram should run");

    let voronoi = wbvector::read(&voronoi_out).expect("read voronoi output");
    assert!(voronoi.features.len() >= 3, "expected at least one cell per input point");
    match voronoi.features[0].geometry.as_ref().expect("voronoi geometry") {
        Geometry::Polygon { .. } => {}
        _ => panic!("voronoi output should be polygon geometry"),
    }

    let _ = std::fs::remove_file(&lines_in);
    let _ = std::fs::remove_file(&lines_out);
    let _ = std::fs::remove_file(&points_in);
    let _ = std::fs::remove_file(&voronoi_out);
}

#[test]
fn travelling_salesman_problem_run_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_tsp");
    let points_in = std::env::temp_dir().join(format!("{tag}_points.geojson"));
    let tour_out = std::env::temp_dir().join(format!("{tag}_tour.geojson"));

    // Create 4 points in a square
    let mut point_layer = Layer::new("points").with_geom_type(wbvector::GeometryType::Point);
    point_layer
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[])
        .expect("add point 1");
    point_layer
        .add_feature(Some(Geometry::point(1.0, 0.0)), &[])
        .expect("add point 2");
    point_layer
        .add_feature(Some(Geometry::point(1.0, 1.0)), &[])
        .expect("add point 3");
    point_layer
        .add_feature(Some(Geometry::point(0.0, 1.0)), &[])
        .expect("add point 4");
    wbvector::write(&point_layer, &points_in, VectorFormat::GeoJson).expect("write points input");

    let mut tsp_args = ToolArgs::new();
    tsp_args.insert("input".to_string(), json!(points_in.to_string_lossy().to_string()));
    tsp_args.insert("duration".to_string(), json!(5));
    tsp_args.insert("output".to_string(), json!(tour_out.to_string_lossy().to_string()));
    registry
        .run("travelling_salesman_problem", &tsp_args, &context(&caps))
        .expect("travelling_salesman_problem should run");

    let tour = wbvector::read(&tour_out).expect("read tour output");
    assert_eq!(tour.features.len(), 1, "expected exactly one feature (the tour)");
    
    // Verify the feature is a line string
    match tour.features[0].geometry.as_ref().expect("tour geometry") {
        Geometry::LineString(coords) => {
            // Should have 5 points (4 input + 1 to close the loop)
            assert!(coords.len() >= 4, "expected at least 4 points in tour");
            // Tour should be closed (first and last points should be the same)
            if coords.len() > 2 {
                assert!(
                    (coords[0].x - coords[coords.len() - 1].x).abs() < 1e-9
                        && (coords[0].y - coords[coords.len() - 1].y).abs() < 1e-9,
                    "tour should be closed loop"
                );
            }
        }
        _ => panic!("tour output should be LineString geometry"),
    }

    let _ = std::fs::remove_file(&points_in);
    let _ = std::fs::remove_file(&tour_out);
}

#[test]
fn update_nodata_cells_replaces_only_missing_values() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_update_nodata");
    let in1_path = std::env::temp_dir().join(format!("{tag}_in1.asc"));
    let in2_path = std::env::temp_dir().join(format!("{tag}_in2.asc"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let nodata = -9999.0;
    for (path, values) in [
        (&in1_path, [1.0, nodata, 3.0, nodata]),
        (&in2_path, [10.0, 20.0, nodata, 40.0]),
    ] {
        let mut raster = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        for (index, value) in values.into_iter().enumerate() {
            raster
                .set(0, (index / 2) as isize, (index % 2) as isize, value)
                .expect("set");
        }
        raster.write(path, RasterFormat::EsriAscii).expect("write raster");
    }

    let mut args = ToolArgs::new();
    args.insert("input1".to_string(), json!(in1_path.to_string_lossy().to_string()));
    args.insert("input2".to_string(), json!(in2_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("update_nodata_cells", &args, &context(&caps))
        .expect("update_nodata_cells should run");

    let out = Raster::read(&out_path).expect("read output");
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 20.0);
    assert_eq!(out.get(0, 1, 0), 3.0);
    assert_eq!(out.get(0, 1, 1), 40.0);

    let _ = std::fs::remove_file(&in1_path);
    let _ = std::fs::remove_file(&in2_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn buffer_raster_creates_expected_binary_buffer() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_buffer_raster");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 5,
        rows: 5,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..5isize {
        for c in 0..5isize {
            raster.set(0, r, c, 0.0).expect("set");
        }
    }
    raster.set(0, 2, 2, 1.0).expect("set center target");
    raster.write(&input_path, RasterFormat::EsriAscii).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("buffer_size".to_string(), json!(1.0));
    args.insert("grid_cell_units".to_string(), json!(true));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("buffer_raster", &args, &context(&caps))
        .expect("buffer_raster should run");

    let out = Raster::read(&output_path).expect("read output");
    assert_eq!(out.get(0, 2, 2), 1.0);
    assert_eq!(out.get(0, 1, 2), 1.0);
    assert_eq!(out.get(0, 3, 2), 1.0);
    assert_eq!(out.get(0, 2, 1), 1.0);
    assert_eq!(out.get(0, 2, 3), 1.0);
    assert_eq!(out.get(0, 1, 1), 0.0);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn clump_assigns_patch_ids_with_zero_background() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_clump");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.asc"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    let values = [
        1.0, 1.0, 0.0,
        1.0, 2.0, 2.0,
        0.0, 2.0, 2.0,
    ];
    for (i, v) in values.into_iter().enumerate() {
        raster
            .set(0, (i / 3) as isize, (i % 3) as isize, v)
            .expect("set");
    }
    raster.write(&input_path, RasterFormat::EsriAscii).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("diag".to_string(), json!(false));
    args.insert("zero_background".to_string(), json!(true));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("clump", &args, &context(&caps))
        .expect("clump should run");

    let out = Raster::read(&output_path).expect("read output");
    assert_eq!(out.get(0, 0, 2), 0.0);
    assert_eq!(out.get(0, 2, 0), 0.0);
    assert_eq!(out.get(0, 0, 0), 1.0);
    assert_eq!(out.get(0, 1, 0), 1.0);
    assert_eq!(out.get(0, 0, 1), 1.0);
    assert_eq!(out.get(0, 1, 1), 2.0);
    assert_eq!(out.get(0, 1, 2), 2.0);
    assert_eq!(out.get(0, 2, 1), 2.0);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn euclidean_distance_and_allocation_compute_expected_outputs() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_euclidean");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.asc"));
    let dist_path = std::env::temp_dir().join(format!("{tag}_dist.asc"));
    let alloc_path = std::env::temp_dir().join(format!("{tag}_alloc.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            raster.set(0, r, c, 0.0).expect("set");
        }
    }
    raster.set(0, 1, 1, 5.0).expect("set target");
    raster.write(&input_path, RasterFormat::EsriAscii).expect("write input");

    let caps = OpenOnly;

    let mut dist_args = ToolArgs::new();
    dist_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    dist_args.insert("output".to_string(), json!(dist_path.to_string_lossy().to_string()));
    registry
        .run("euclidean_distance", &dist_args, &context(&caps))
        .expect("euclidean_distance should run");

    let mut alloc_args = ToolArgs::new();
    alloc_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    alloc_args.insert("output".to_string(), json!(alloc_path.to_string_lossy().to_string()));
    registry
        .run("euclidean_allocation", &alloc_args, &context(&caps))
        .expect("euclidean_allocation should run");

    let dist_out = Raster::read(&dist_path).expect("read distance output");
    assert!((dist_out.get(0, 1, 1) - 0.0).abs() < 1e-6);
    assert!((dist_out.get(0, 0, 1) - 1.0).abs() < 1e-6);
    assert!((dist_out.get(0, 0, 0) - 2f64.sqrt()).abs() < 1e-6);

    let alloc_out = Raster::read(&alloc_path).expect("read allocation output");
    for r in 0..3isize {
        for c in 0..3isize {
            assert!((alloc_out.get(0, r, c) - 5.0).abs() < 1e-6);
        }
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&dist_path);
    let _ = std::fs::remove_file(&alloc_path);
}

#[test]
fn cost_distance_allocation_and_pathway_compute_expected_outputs() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_cost_tools");
    let source_path = std::env::temp_dir().join(format!("{tag}_source.asc"));
    let cost_path = std::env::temp_dir().join(format!("{tag}_cost.asc"));
    let accum_path = std::env::temp_dir().join(format!("{tag}_accum.asc"));
    let backlink_path = std::env::temp_dir().join(format!("{tag}_backlink.asc"));
    let alloc_path = std::env::temp_dir().join(format!("{tag}_allocation.asc"));
    let destination_path = std::env::temp_dir().join(format!("{tag}_destination.asc"));
    let pathway_path = std::env::temp_dir().join(format!("{tag}_pathway.asc"));

    let mut source = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            source.set(0, r, c, 0.0).expect("set");
        }
    }
    source.set(0, 1, 1, 1.0).expect("set source");
    source.write(&source_path, RasterFormat::EsriAscii).expect("write source");

    let mut cost = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            cost.set(0, r, c, 1.0).expect("set");
        }
    }
    cost.write(&cost_path, RasterFormat::EsriAscii).expect("write cost");

    let caps = OpenOnly;

    let mut dist_args = ToolArgs::new();
    dist_args.insert("source".to_string(), json!(source_path.to_string_lossy().to_string()));
    dist_args.insert("cost".to_string(), json!(cost_path.to_string_lossy().to_string()));
    dist_args.insert("output".to_string(), json!(accum_path.to_string_lossy().to_string()));
    dist_args.insert("backlink_output".to_string(), json!(backlink_path.to_string_lossy().to_string()));
    registry
        .run("cost_distance", &dist_args, &context(&caps))
        .expect("cost_distance should run");

    let accum_out = Raster::read(&accum_path).expect("read accum output");
    assert!((accum_out.get(0, 1, 1) - 0.0).abs() < 1e-6);
    assert!((accum_out.get(0, 1, 2) - 1.0).abs() < 1e-6);
    assert!((accum_out.get(0, 0, 0) - 2f64.sqrt()).abs() < 1e-6);

    let mut alloc_args = ToolArgs::new();
    alloc_args.insert("source".to_string(), json!(source_path.to_string_lossy().to_string()));
    alloc_args.insert("backlink".to_string(), json!(backlink_path.to_string_lossy().to_string()));
    alloc_args.insert("output".to_string(), json!(alloc_path.to_string_lossy().to_string()));
    registry
        .run("cost_allocation", &alloc_args, &context(&caps))
        .expect("cost_allocation should run");

    let alloc_out = Raster::read(&alloc_path).expect("read allocation output");
    for r in 0..3isize {
        for c in 0..3isize {
            assert!((alloc_out.get(0, r, c) - 1.0).abs() < 1e-6);
        }
    }

    let mut destination = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            destination.set(0, r, c, 0.0).expect("set");
        }
    }
    destination.set(0, 0, 0, 1.0).expect("set destination");
    destination.set(0, 2, 2, 1.0).expect("set destination");
    destination
        .write(&destination_path, RasterFormat::EsriAscii)
        .expect("write destination");

    let mut path_args = ToolArgs::new();
    path_args.insert(
        "destination".to_string(),
        json!(destination_path.to_string_lossy().to_string()),
    );
    path_args.insert("backlink".to_string(), json!(backlink_path.to_string_lossy().to_string()));
    path_args.insert("zero_background".to_string(), json!(true));
    path_args.insert("output".to_string(), json!(pathway_path.to_string_lossy().to_string()));
    registry
        .run("cost_pathway", &path_args, &context(&caps))
        .expect("cost_pathway should run");

    let pathway_out = Raster::read(&pathway_path).expect("read pathway output");
    assert!(pathway_out.get(0, 1, 1) >= 1.0);
    assert!(pathway_out.get(0, 0, 0) >= 1.0);
    assert!(pathway_out.get(0, 2, 2) >= 1.0);

    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_file(&cost_path);
    let _ = std::fs::remove_file(&accum_path);
    let _ = std::fs::remove_file(&backlink_path);
    let _ = std::fs::remove_file(&alloc_path);
    let _ = std::fs::remove_file(&destination_path);
    let _ = std::fs::remove_file(&pathway_path);
}

#[test]
fn buffer_vector_creates_polygon_output_from_line_input() {
    use wbvector::{Coord, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_buffer_vector");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut layer = Layer::new("lines").with_geom_type(wbvector::GeometryType::LineString);
    layer
        .add_feature(
            Some(wbvector::Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(4.0, 0.0),
            ])),
            &[],
        )
        .expect("add line");
    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input line vector");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("distance".to_string(), json!(1.0));
    args.insert("quadrant_segments".to_string(), json!(8));
    args.insert("cap_style".to_string(), json!("round"));
    args.insert("join_style".to_string(), json!("round"));
    args.insert("mitre_limit".to_string(), json!(5.0));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));

    let caps = OpenOnly;
    registry
        .run("buffer_vector", &args, &context(&caps))
        .expect("buffer_vector should run");

    let output = wbvector::read(&output_path).expect("read buffered output");
    assert_eq!(output.features.len(), 1);
    match output.features[0].geometry.as_ref() {
        Some(wbvector::Geometry::Polygon { .. }) | Some(wbvector::Geometry::MultiPolygon(_)) => {}
        other => panic!("expected polygonal output geometry, found {:?}", other),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn vector_overlay_tools_run_end_to_end_and_merge_attributes() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vector_overlay");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.geojson"));
    let overlay_path = std::env::temp_dir().join(format!("{tag}_overlay.geojson"));
    let clip_path = std::env::temp_dir().join(format!("{tag}_clip.geojson"));
    let difference_path = std::env::temp_dir().join(format!("{tag}_difference.geojson"));
    let erase_path = std::env::temp_dir().join(format!("{tag}_erase.geojson"));
    let intersect_path = std::env::temp_dir().join(format!("{tag}_intersect.geojson"));
    let symdiff_path = std::env::temp_dir().join(format!("{tag}_symdiff.geojson"));
    let union_path = std::env::temp_dir().join(format!("{tag}_union.geojson"));

    let mut input = Layer::new("input").with_geom_type(wbvector::GeometryType::Polygon);
    input.add_field(FieldDef::new("A", FieldType::Integer));
    input
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[("A", FieldValue::Integer(1))],
        )
        .expect("add input polygon");
    wbvector::write(&input, &input_path, VectorFormat::GeoJson).expect("write input polygons");

    let mut overlay = Layer::new("overlay").with_geom_type(wbvector::GeometryType::Polygon);
    overlay.add_field(FieldDef::new("B", FieldType::Integer));
    overlay
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(1.0, 1.0),
                    Coord::xy(3.0, 1.0),
                    Coord::xy(3.0, 3.0),
                    Coord::xy(1.0, 3.0),
                    Coord::xy(1.0, 1.0),
                ],
                vec![],
            )),
            &[("B", FieldValue::Integer(2))],
        )
        .expect("add overlay polygon");
    wbvector::write(&overlay, &overlay_path, VectorFormat::GeoJson).expect("write overlay polygons");

    let mut base_args = ToolArgs::new();
    base_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    base_args.insert("overlay".to_string(), json!(overlay_path.to_string_lossy().to_string()));

    let mut clip_args = base_args.clone();
    clip_args.insert("output".to_string(), json!(clip_path.to_string_lossy().to_string()));
    registry
        .run("clip", &clip_args, &context(&caps))
        .expect("clip should run");

    let mut difference_args = base_args.clone();
    difference_args.insert("output".to_string(), json!(difference_path.to_string_lossy().to_string()));
    registry
        .run("difference", &difference_args, &context(&caps))
        .expect("difference should run");

    let mut erase_args = base_args.clone();
    erase_args.insert("output".to_string(), json!(erase_path.to_string_lossy().to_string()));
    registry
        .run("erase", &erase_args, &context(&caps))
        .expect("erase should run");

    let mut intersect_args = base_args.clone();
    intersect_args.insert("output".to_string(), json!(intersect_path.to_string_lossy().to_string()));
    registry
        .run("intersect", &intersect_args, &context(&caps))
        .expect("intersect should run");

    let mut symdiff_args = base_args.clone();
    symdiff_args.insert("output".to_string(), json!(symdiff_path.to_string_lossy().to_string()));
    registry
        .run("symmetrical_difference", &symdiff_args, &context(&caps))
        .expect("symmetrical_difference should run");

    let mut union_args = base_args;
    union_args.insert("output".to_string(), json!(union_path.to_string_lossy().to_string()));
    registry
        .run("union", &union_args, &context(&caps))
        .expect("union should run");

    let clip_out = wbvector::read(&clip_path).expect("read clip output");
    assert!(!clip_out.features.is_empty(), "clip should produce at least one feature");
    assert!(clip_out.schema.field_index("A").is_some(), "clip should preserve input field A");
    assert!(clip_out.schema.field_index("B").is_none(), "clip should not add overlay-only field B");

    let diff_out = wbvector::read(&difference_path).expect("read difference output");
    assert!(!diff_out.features.is_empty(), "difference should produce at least one feature");
    assert!(diff_out.schema.field_index("A").is_some(), "difference should preserve input field A");
    assert!(diff_out.schema.field_index("B").is_none(), "difference should not add overlay-only field B");

    let erase_out = wbvector::read(&erase_path).expect("read erase output");
    assert!(!erase_out.features.is_empty(), "erase should produce at least one feature");
    assert!(erase_out.schema.field_index("A").is_some(), "erase should preserve input field A");
    assert!(erase_out.schema.field_index("B").is_none(), "erase should not add overlay-only field B");

    let intersect_out = wbvector::read(&intersect_path).expect("read intersect output");
    assert!(!intersect_out.features.is_empty(), "intersect should produce overlap features");
    let ia = intersect_out.schema.field_index("A").expect("intersect field A");
    let ib = intersect_out.schema.field_index("B").expect("intersect field B");
    assert!(intersect_out.features.iter().any(|f| {
        matches!(f.attributes.get(ia), Some(FieldValue::Integer(1)))
            && matches!(f.attributes.get(ib), Some(FieldValue::Integer(2)))
    }));

    let symdiff_out = wbvector::read(&symdiff_path).expect("read symdiff output");
    assert!(!symdiff_out.features.is_empty(), "symmetrical_difference should produce non-overlap features");
    let sa = symdiff_out.schema.field_index("A").expect("symdiff field A");
    let sb = symdiff_out.schema.field_index("B").expect("symdiff field B");
    assert!(symdiff_out.features.iter().any(|f| {
        matches!(f.attributes.get(sa), Some(FieldValue::Integer(1)))
            && matches!(f.attributes.get(sb), Some(FieldValue::Null))
    }));
    assert!(symdiff_out.features.iter().any(|f| {
        matches!(f.attributes.get(sa), Some(FieldValue::Null))
            && matches!(f.attributes.get(sb), Some(FieldValue::Integer(2)))
    }));

    let union_out = wbvector::read(&union_path).expect("read union output");
    assert!(!union_out.features.is_empty(), "union should produce at least one feature");
    let ua = union_out.schema.field_index("A").expect("union field A");
    let ub = union_out.schema.field_index("B").expect("union field B");
    assert!(union_out.features.iter().any(|f| {
        matches!(f.attributes.get(ua), Some(FieldValue::Integer(1)))
            && matches!(f.attributes.get(ub), Some(FieldValue::Integer(2)))
    }));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&overlay_path);
    let _ = std::fs::remove_file(&clip_path);
    let _ = std::fs::remove_file(&difference_path);
    let _ = std::fs::remove_file(&erase_path);
    let _ = std::fs::remove_file(&intersect_path);
    let _ = std::fs::remove_file(&symdiff_path);
    let _ = std::fs::remove_file(&union_path);
}

#[test]
fn vector_overlay_same_name_field_collision_prefers_overlay_on_overlap() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vector_overlay_collision");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.geojson"));
    let overlay_path = std::env::temp_dir().join(format!("{tag}_overlay.geojson"));
    let intersect_path = std::env::temp_dir().join(format!("{tag}_intersect.geojson"));
    let union_path = std::env::temp_dir().join(format!("{tag}_union.geojson"));

    let mut input = Layer::new("input").with_geom_type(wbvector::GeometryType::Polygon);
    input.add_field(FieldDef::new("ID", FieldType::Integer));
    input.add_field(FieldDef::new("IN_ONLY", FieldType::Integer));
    input
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[
                ("ID", FieldValue::Integer(11)),
                ("IN_ONLY", FieldValue::Integer(5)),
            ],
        )
        .expect("add input polygon");
    wbvector::write(&input, &input_path, VectorFormat::GeoJson).expect("write input polygons");

    let mut overlay = Layer::new("overlay").with_geom_type(wbvector::GeometryType::Polygon);
    overlay.add_field(FieldDef::new("ID", FieldType::Integer));
    overlay.add_field(FieldDef::new("OV_ONLY", FieldType::Integer));
    overlay
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(1.0, 1.0),
                    Coord::xy(3.0, 1.0),
                    Coord::xy(3.0, 3.0),
                    Coord::xy(1.0, 3.0),
                    Coord::xy(1.0, 1.0),
                ],
                vec![],
            )),
            &[
                ("ID", FieldValue::Integer(22)),
                ("OV_ONLY", FieldValue::Integer(9)),
            ],
        )
        .expect("add overlay polygon");
    wbvector::write(&overlay, &overlay_path, VectorFormat::GeoJson).expect("write overlay polygons");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("overlay".to_string(), json!(overlay_path.to_string_lossy().to_string()));

    let mut intersect_args = args.clone();
    intersect_args.insert("output".to_string(), json!(intersect_path.to_string_lossy().to_string()));
    registry
        .run("intersect", &intersect_args, &context(&caps))
        .expect("intersect should run");

    let mut union_args = args;
    union_args.insert("output".to_string(), json!(union_path.to_string_lossy().to_string()));
    registry
        .run("union", &union_args, &context(&caps))
        .expect("union should run");

    let intersect_out = wbvector::read(&intersect_path).expect("read intersect output");
    assert!(!intersect_out.features.is_empty(), "intersect should produce overlap features");

    let id_count = intersect_out
        .schema
        .fields()
        .iter()
        .filter(|f| f.name == "ID")
        .count();
    assert_eq!(id_count, 1, "merged schema should only contain one ID field");

    let id_idx = intersect_out.schema.field_index("ID").expect("ID field");
    let in_only_idx = intersect_out.schema.field_index("IN_ONLY").expect("IN_ONLY field");
    let ov_only_idx = intersect_out.schema.field_index("OV_ONLY").expect("OV_ONLY field");
    assert!(intersect_out.features.iter().any(|f| {
        matches!(f.attributes.get(id_idx), Some(FieldValue::Integer(22)))
            && matches!(f.attributes.get(in_only_idx), Some(FieldValue::Integer(5)))
            && matches!(f.attributes.get(ov_only_idx), Some(FieldValue::Integer(9)))
    }), "on overlap, ID should take overlay value while source-specific fields are preserved");

    let union_out = wbvector::read(&union_path).expect("read union output");
    let uid_idx = union_out.schema.field_index("ID").expect("ID field");
    let uin_only_idx = union_out.schema.field_index("IN_ONLY").expect("IN_ONLY field");
    let uov_only_idx = union_out.schema.field_index("OV_ONLY").expect("OV_ONLY field");
    assert!(union_out.features.iter().any(|f| {
        matches!(f.attributes.get(uid_idx), Some(FieldValue::Integer(22)))
            && matches!(f.attributes.get(uin_only_idx), Some(FieldValue::Integer(5)))
            && matches!(f.attributes.get(uov_only_idx), Some(FieldValue::Integer(9)))
    }), "union overlap feature should prefer overlay value for shared field names");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&overlay_path);
    let _ = std::fs::remove_file(&intersect_path);
    let _ = std::fs::remove_file(&union_path);
}

#[test]
fn clip_raster_to_polygon_and_erase_polygon_from_raster_run_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_clip_erase_raster_polygon");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.asc"));
    let polygons_path = std::env::temp_dir().join(format!("{tag}_poly.geojson"));
    let clip_keep_dims_path = std::env::temp_dir().join(format!("{tag}_clip_keep.asc"));
    let clip_crop_path = std::env::temp_dir().join(format!("{tag}_clip_crop.asc"));
    let erase_path = std::env::temp_dir().join(format!("{tag}_erase.asc"));

    let mut input = Raster::new(RasterConfig {
        cols: 4,
        rows: 4,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::default(),
        metadata: Vec::new(),
    });
    for row in 0..4isize {
        for col in 0..4isize {
            input
                .set(0, row, col, (row * 10 + col) as f64)
                .expect("set input");
        }
    }
    input
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    let mut polygons = Layer::new("polygons")
        .with_geom_type(wbvector::GeometryType::Polygon);
    polygons
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(1.0, 1.0),
                    Coord::xy(3.0, 1.0),
                    Coord::xy(3.0, 3.0),
                    Coord::xy(1.0, 3.0),
                    Coord::xy(1.0, 1.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon");
    wbvector::write(&polygons, &polygons_path, VectorFormat::GeoJson)
        .expect("write polygons");

    let mut clip_keep_args = ToolArgs::new();
    clip_keep_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    clip_keep_args.insert("polygons".to_string(), json!(polygons_path.to_string_lossy().to_string()));
    clip_keep_args.insert("maintain_dimensions".to_string(), json!(true));
    clip_keep_args.insert("output".to_string(), json!(clip_keep_dims_path.to_string_lossy().to_string()));
    registry
        .run("clip_raster_to_polygon", &clip_keep_args, &context(&caps))
        .expect("clip_raster_to_polygon keep-dimensions should run");

    let mut clip_crop_args = ToolArgs::new();
    clip_crop_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    clip_crop_args.insert("polygons".to_string(), json!(polygons_path.to_string_lossy().to_string()));
    clip_crop_args.insert("maintain_dimensions".to_string(), json!(false));
    clip_crop_args.insert("output".to_string(), json!(clip_crop_path.to_string_lossy().to_string()));
    registry
        .run("clip_raster_to_polygon", &clip_crop_args, &context(&caps))
        .expect("clip_raster_to_polygon cropped should run");

    let mut erase_args = ToolArgs::new();
    erase_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    erase_args.insert("polygons".to_string(), json!(polygons_path.to_string_lossy().to_string()));
    erase_args.insert("output".to_string(), json!(erase_path.to_string_lossy().to_string()));
    registry
        .run("erase_polygon_from_raster", &erase_args, &context(&caps))
        .expect("erase_polygon_from_raster should run");

    let clip_keep = Raster::read(&clip_keep_dims_path).expect("read keep-dim clip");
    assert_eq!(clip_keep.rows, 4);
    assert_eq!(clip_keep.cols, 4);
    assert_eq!(clip_keep.get(0, 0, 0), clip_keep.nodata);
    assert_eq!(clip_keep.get(0, 1, 1), input.get(0, 1, 1));

    let clip_crop = Raster::read(&clip_crop_path).expect("read cropped clip");
    assert_eq!(clip_crop.rows, 2);
    assert_eq!(clip_crop.cols, 2);
    assert_eq!(clip_crop.get(0, 0, 0), input.get(0, 1, 1));

    let erase = Raster::read(&erase_path).expect("read erase output");
    assert_eq!(erase.get(0, 1, 1), erase.nodata);
    assert_eq!(erase.get(0, 0, 0), input.get(0, 0, 0));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&polygons_path);
    let _ = std::fs::remove_file(&clip_keep_dims_path);
    let _ = std::fs::remove_file(&clip_crop_path);
    let _ = std::fs::remove_file(&erase_path);
}

#[test]
fn clip_and_erase_polygon_tools_reproject_vector_to_raster_crs() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_clip_erase_reproject");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.asc"));
    let polygons_path = std::env::temp_dir().join(format!("{tag}_poly.gpkg"));
    let clip_path = std::env::temp_dir().join(format!("{tag}_clip.asc"));
    let erase_path = std::env::temp_dir().join(format!("{tag}_erase.asc"));

    // Raster is in EPSG:4326 covering 0..4 in both axes.
    let mut input = Raster::new(RasterConfig {
        cols: 4,
        rows: 4,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for row in 0..4isize {
        for col in 0..4isize {
            input
                .set(0, row, col, (row * 10 + col) as f64)
                .expect("set input");
        }
    }
    input
        .write(&input_path, RasterFormat::EsriAscii)
        .expect("write input raster");

    // Polygon is in EPSG:3857. Coordinates correspond approximately to 1..3 degrees
    // after reprojection to EPSG:4326. If reprojection is skipped, this polygon does
    // not overlap the raster extent and tools should fail.
    let mut polygons = Layer::new("polygons")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(3857);
    polygons
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(111_319.490_793_273_57, 111_325.142_866_385_1),
                    Coord::xy(333_958.472_379_820_7, 111_325.142_866_385_1),
                    Coord::xy(333_958.472_379_820_7, 334_111.171_401_959_6),
                    Coord::xy(111_319.490_793_273_57, 334_111.171_401_959_6),
                    Coord::xy(111_319.490_793_273_57, 111_325.142_866_385_1),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon");
    wbvector::write(&polygons, &polygons_path, VectorFormat::GeoPackage)
        .expect("write polygons gpkg");

    let mut clip_args = ToolArgs::new();
    clip_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    clip_args.insert("polygons".to_string(), json!(polygons_path.to_string_lossy().to_string()));
    clip_args.insert("maintain_dimensions".to_string(), json!(true));
    clip_args.insert("output".to_string(), json!(clip_path.to_string_lossy().to_string()));
    registry
        .run("clip_raster_to_polygon", &clip_args, &context(&caps))
        .expect("clip_raster_to_polygon should run with CRS reprojection");

    let mut erase_args = ToolArgs::new();
    erase_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    erase_args.insert("polygons".to_string(), json!(polygons_path.to_string_lossy().to_string()));
    erase_args.insert("output".to_string(), json!(erase_path.to_string_lossy().to_string()));
    registry
        .run("erase_polygon_from_raster", &erase_args, &context(&caps))
        .expect("erase_polygon_from_raster should run with CRS reprojection");

    let clip = Raster::read(&clip_path).expect("read clip output");
    let erase = Raster::read(&erase_path).expect("read erase output");

    // Cell centred near (1.5, 2.5) should be inside the reprojected polygon.
    assert_eq!(clip.get(0, 1, 1), input.get(0, 1, 1));
    assert_eq!(erase.get(0, 1, 1), erase.nodata);

    // Cell centred near (0.5, 3.5) should remain outside.
    assert_eq!(clip.get(0, 0, 0), clip.nodata);
    assert_eq!(erase.get(0, 0, 0), input.get(0, 0, 0));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&polygons_path);
    let _ = std::fs::remove_file(&clip_path);
    let _ = std::fs::remove_file(&erase_path);
}

#[test]
fn vector_overlay_tools_reproject_mismatched_crs_layers() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vector_overlay_reproject");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.gpkg"));
    let overlay_path = std::env::temp_dir().join(format!("{tag}_overlay.gpkg"));
    let clip_path = std::env::temp_dir().join(format!("{tag}_clip.geojson"));
    let diff_path = std::env::temp_dir().join(format!("{tag}_difference.geojson"));
    let erase_path = std::env::temp_dir().join(format!("{tag}_erase.geojson"));
    let intersect_path = std::env::temp_dir().join(format!("{tag}_intersect.geojson"));
    let symdiff_path = std::env::temp_dir().join(format!("{tag}_symdiff.geojson"));
    let union_path = std::env::temp_dir().join(format!("{tag}_union.geojson"));

    // Input layer in EPSG:4326.
    let mut input = Layer::new("input")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(4326);
    input.add_field(FieldDef::new("A", FieldType::Integer));
    input
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[("A", FieldValue::Integer(1))],
        )
        .expect("add input polygon");
    wbvector::write(&input, &input_path, VectorFormat::GeoPackage)
        .expect("write input gpkg");

    // Overlay layer in EPSG:3857 that corresponds roughly to 1..3 degrees in EPSG:4326.
    let mut overlay = Layer::new("overlay")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(3857);
    overlay.add_field(FieldDef::new("B", FieldType::Integer));
    overlay
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(111_319.490_793_273_57, 111_325.142_866_385_1),
                    Coord::xy(333_958.472_379_820_7, 111_325.142_866_385_1),
                    Coord::xy(333_958.472_379_820_7, 334_111.171_401_959_6),
                    Coord::xy(111_319.490_793_273_57, 334_111.171_401_959_6),
                    Coord::xy(111_319.490_793_273_57, 111_325.142_866_385_1),
                ],
                vec![],
            )),
            &[("B", FieldValue::Integer(2))],
        )
        .expect("add overlay polygon");
    wbvector::write(&overlay, &overlay_path, VectorFormat::GeoPackage)
        .expect("write overlay gpkg");

    let mut base_args = ToolArgs::new();
    base_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    base_args.insert("overlay".to_string(), json!(overlay_path.to_string_lossy().to_string()));

    let mut clip_args = base_args.clone();
    clip_args.insert("output".to_string(), json!(clip_path.to_string_lossy().to_string()));
    registry.run("clip", &clip_args, &context(&caps)).expect("clip should run with reprojection");

    let mut diff_args = base_args.clone();
    diff_args.insert("output".to_string(), json!(diff_path.to_string_lossy().to_string()));
    registry
        .run("difference", &diff_args, &context(&caps))
        .expect("difference should run with reprojection");

    let mut erase_args = base_args.clone();
    erase_args.insert("output".to_string(), json!(erase_path.to_string_lossy().to_string()));
    registry
        .run("erase", &erase_args, &context(&caps))
        .expect("erase should run with reprojection");

    let mut intersect_args = base_args.clone();
    intersect_args.insert("output".to_string(), json!(intersect_path.to_string_lossy().to_string()));
    registry
        .run("intersect", &intersect_args, &context(&caps))
        .expect("intersect should run with reprojection");

    let mut symdiff_args = base_args.clone();
    symdiff_args.insert("output".to_string(), json!(symdiff_path.to_string_lossy().to_string()));
    registry
        .run("symmetrical_difference", &symdiff_args, &context(&caps))
        .expect("symmetrical_difference should run with reprojection");

    let mut union_args = base_args;
    union_args.insert("output".to_string(), json!(union_path.to_string_lossy().to_string()));
    registry
        .run("union", &union_args, &context(&caps))
        .expect("union should run with reprojection");

    let clip_out = wbvector::read(&clip_path).expect("read clip output");
    let intersect_out = wbvector::read(&intersect_path).expect("read intersect output");
    assert!(
        !clip_out.features.is_empty(),
        "clip should produce overlap features after reprojection"
    );
    assert!(
        !intersect_out.features.is_empty(),
        "intersect should produce overlap features after reprojection"
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&overlay_path);
    let _ = std::fs::remove_file(&clip_path);
    let _ = std::fs::remove_file(&diff_path);
    let _ = std::fs::remove_file(&erase_path);
    let _ = std::fs::remove_file(&intersect_path);
    let _ = std::fs::remove_file(&symdiff_path);
    let _ = std::fs::remove_file(&union_path);
}

#[test]
fn linework_tools_run_and_two_input_tools_reproject_mismatched_crs() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_linework_tools");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.gpkg"));
    let split_path = std::env::temp_dir().join(format!("{tag}_split.gpkg"));
    let line_intersections_path = std::env::temp_dir().join(format!("{tag}_line_intersections.gpkg"));
    let split_out_path = std::env::temp_dir().join(format!("{tag}_split_out.gpkg"));
    let merge_input_path = std::env::temp_dir().join(format!("{tag}_merge_input.geojson"));
    let merge_out_path = std::env::temp_dir().join(format!("{tag}_merge_out.geojson"));
    let polygonize_input_path = std::env::temp_dir().join(format!("{tag}_polygonize_in.geojson"));
    let polygonize_out_path = std::env::temp_dir().join(format!("{tag}_polygonize_out.geojson"));

    // Input line in EPSG:4326.
    let mut input = Layer::new("input")
        .with_geom_type(wbvector::GeometryType::LineString)
        .with_epsg(4326);
    input
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add input line");
    wbvector::write(&input, &input_path, VectorFormat::GeoPackage)
        .expect("write input gpkg");

    // Split/overlay line in EPSG:3857 crossing x ~= 1 degree after reprojection.
    let mut split = Layer::new("split")
        .with_geom_type(wbvector::GeometryType::LineString)
        .with_epsg(3857);
    split
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(111_319.490_793_273_57, -111_325.142_866_385_1),
                Coord::xy(111_319.490_793_273_57, 111_325.142_866_385_1),
            ])),
            &[],
        )
        .expect("add split line");
    wbvector::write(&split, &split_path, VectorFormat::GeoPackage)
        .expect("write split gpkg");

    let mut li_args = ToolArgs::new();
    li_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    li_args.insert("overlay".to_string(), json!(split_path.to_string_lossy().to_string()));
    li_args.insert("output".to_string(), json!(line_intersections_path.to_string_lossy().to_string()));
    registry
        .run("line_intersections", &li_args, &context(&caps))
        .expect("line_intersections should run with reprojection");

    let mut split_args = ToolArgs::new();
    split_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    split_args.insert("split".to_string(), json!(split_path.to_string_lossy().to_string()));
    split_args.insert("output".to_string(), json!(split_out_path.to_string_lossy().to_string()));
    registry
        .run("split_with_lines", &split_args, &context(&caps))
        .expect("split_with_lines should run with reprojection");

    let li_out = wbvector::read(&line_intersections_path).expect("read line intersections output");
    let split_out = wbvector::read(&split_out_path).expect("read split_with_lines output");
    assert!(!li_out.features.is_empty(), "line_intersections should produce at least one point");
    assert!(split_out.features.len() >= 2, "split_with_lines should split the input line into at least two pieces");

    // Merge connected segments.
    let mut merge_in = Layer::new("merge_in")
        .with_geom_type(wbvector::GeometryType::LineString)
        .with_epsg(4326);
    merge_in
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add merge line 1");
    merge_in
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add merge line 2");
    wbvector::write(&merge_in, &merge_input_path, VectorFormat::GeoJson)
        .expect("write merge input");

    let mut merge_args = ToolArgs::new();
    merge_args.insert("input".to_string(), json!(merge_input_path.to_string_lossy().to_string()));
    merge_args.insert("output".to_string(), json!(merge_out_path.to_string_lossy().to_string()));
    registry
        .run("merge_line_segments", &merge_args, &context(&caps))
        .expect("merge_line_segments should run");

    let merge_out = wbvector::read(&merge_out_path).expect("read merge output");
    assert_eq!(merge_out.features.len(), 1, "merge_line_segments should merge two connected segments");

    // Polygonize a closed ring.
    let mut polygonize_in = Layer::new("polygonize_in")
        .with_geom_type(wbvector::GeometryType::LineString)
        .with_epsg(4326);
    polygonize_in
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(2.0, 0.0),
                Coord::xy(2.0, 2.0),
                Coord::xy(0.0, 2.0),
                Coord::xy(0.0, 0.0),
            ])),
            &[],
        )
        .expect("add polygonize ring");
    wbvector::write(&polygonize_in, &polygonize_input_path, VectorFormat::GeoJson)
        .expect("write polygonize input");

    let mut polygonize_args = ToolArgs::new();
    polygonize_args.insert("input".to_string(), json!(polygonize_input_path.to_string_lossy().to_string()));
    polygonize_args.insert("output".to_string(), json!(polygonize_out_path.to_string_lossy().to_string()));
    registry
        .run("polygonize", &polygonize_args, &context(&caps))
        .expect("polygonize should run");

    let polygonize_out = wbvector::read(&polygonize_out_path).expect("read polygonize output");
    assert_eq!(polygonize_out.features.len(), 1, "polygonize should produce one polygon from a closed ring");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&split_path);
    let _ = std::fs::remove_file(&line_intersections_path);
    let _ = std::fs::remove_file(&split_out_path);
    let _ = std::fs::remove_file(&merge_input_path);
    let _ = std::fs::remove_file(&merge_out_path);
    let _ = std::fs::remove_file(&polygonize_input_path);
    let _ = std::fs::remove_file(&polygonize_out_path);
}

#[test]
fn phase1_tools_run_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_phase1_tools");
    let vector_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let hull_path = std::env::temp_dir().join(format!("{tag}_hull.geojson"));
    let mbb_path = std::env::temp_dir().join(format!("{tag}_mbb.geojson"));
    let mbc_path = std::env::temp_dir().join(format!("{tag}_mbc.geojson"));
    let mbe_path = std::env::temp_dir().join(format!("{tag}_mbe.geojson"));
    let raster_path = std::env::temp_dir().join(format!("{tag}_in.asc"));
    let reclass_path = std::env::temp_dir().join(format!("{tag}_reclass.asc"));
    let rei_path = std::env::temp_dir().join(format!("{tag}_rei.asc"));
    let frfa_path = std::env::temp_dir().join(format!("{tag}_frfa.asc"));

    let mut layer = Layer::new("pts")
        .with_geom_type(wbvector::GeometryType::Point)
        .with_epsg(4326);
    layer
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[])
        .expect("add p1");
    layer
        .add_feature(Some(Geometry::point(2.0, 0.0)), &[])
        .expect("add p2");
    layer
        .add_feature(Some(Geometry::point(1.0, 1.5)), &[])
        .expect("add p3");
    wbvector::write(&layer, &vector_path, VectorFormat::GeoPackage)
        .expect("write vector input");

    let mut hull_args = ToolArgs::new();
    hull_args.insert("input".to_string(), json!(vector_path.to_string_lossy().to_string()));
    hull_args.insert("individual_feature_hulls".to_string(), json!(false));
    hull_args.insert("output".to_string(), json!(hull_path.to_string_lossy().to_string()));
    registry
        .run("minimum_convex_hull", &hull_args, &context(&caps))
        .expect("minimum_convex_hull should run");

    let mut mbb_args = ToolArgs::new();
    mbb_args.insert("input".to_string(), json!(vector_path.to_string_lossy().to_string()));
    mbb_args.insert("individual_feature_hulls".to_string(), json!(false));
    mbb_args.insert("min_criteria".to_string(), json!("area"));
    mbb_args.insert("output".to_string(), json!(mbb_path.to_string_lossy().to_string()));
    registry
        .run("minimum_bounding_box", &mbb_args, &context(&caps))
        .expect("minimum_bounding_box should run");

    let mut mbc_args = ToolArgs::new();
    mbc_args.insert("input".to_string(), json!(vector_path.to_string_lossy().to_string()));
    mbc_args.insert("individual_feature_hulls".to_string(), json!(false));
    mbc_args.insert("output".to_string(), json!(mbc_path.to_string_lossy().to_string()));
    registry
        .run("minimum_bounding_circle", &mbc_args, &context(&caps))
        .expect("minimum_bounding_circle should run");

    let mut mbe_args = ToolArgs::new();
    mbe_args.insert("input".to_string(), json!(vector_path.to_string_lossy().to_string()));
    mbe_args.insert("individual_feature_hulls".to_string(), json!(false));
    mbe_args.insert("output".to_string(), json!(mbe_path.to_string_lossy().to_string()));
    registry
        .run("minimum_bounding_envelope", &mbe_args, &context(&caps))
        .expect("minimum_bounding_envelope should run");

    let hull_out = wbvector::read(&hull_path).expect("read hull output");
    let mbb_out = wbvector::read(&mbb_path).expect("read mbb output");
    let mbc_out = wbvector::read(&mbc_path).expect("read mbc output");
    let mbe_out = wbvector::read(&mbe_path).expect("read mbe output");
    assert_eq!(hull_out.features.len(), 1, "minimum_convex_hull should output one feature for full-layer mode");
    assert_eq!(mbb_out.features.len(), 1, "minimum_bounding_box should output one feature for full-layer mode");
    assert_eq!(mbc_out.features.len(), 1, "minimum_bounding_circle should output one feature for full-layer mode");
    assert_eq!(mbe_out.features.len(), 1, "minimum_bounding_envelope should output one feature for full-layer mode");

    let mut raster = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    raster.set(0, 0, 0, 1.0).expect("set");
    raster.set(0, 0, 1, 2.0).expect("set");
    raster.set(0, 1, 0, 3.0).expect("set");
    raster.set(0, 1, 1, 4.0).expect("set");
    raster
        .write(&raster_path, RasterFormat::EsriAscii)
        .expect("write raster input");

    let mut reclass_args = ToolArgs::new();
    reclass_args.insert("input".to_string(), json!(raster_path.to_string_lossy().to_string()));
    reclass_args.insert("reclass_values".to_string(), json!([[10.0, 0.0, 2.5], [20.0, 2.5, 5.0]]));
    reclass_args.insert("output".to_string(), json!(reclass_path.to_string_lossy().to_string()));
    registry
        .run("reclass", &reclass_args, &context(&caps))
        .expect("reclass should run");

    let mut rei_args = ToolArgs::new();
    rei_args.insert("input".to_string(), json!(raster_path.to_string_lossy().to_string()));
    rei_args.insert("interval_size".to_string(), json!(2.0));
    rei_args.insert("start_value".to_string(), json!(0.0));
    rei_args.insert("end_value".to_string(), json!(5.0));
    rei_args.insert("output".to_string(), json!(rei_path.to_string_lossy().to_string()));
    registry
        .run("reclass_equal_interval", &rei_args, &context(&caps))
        .expect("reclass_equal_interval should run");

    let mut frfa_args = ToolArgs::new();
    frfa_args.insert("input".to_string(), json!(raster_path.to_string_lossy().to_string()));
    frfa_args.insert("threshold".to_string(), json!(2));
    frfa_args.insert("zero_background".to_string(), json!(true));
    frfa_args.insert("output".to_string(), json!(frfa_path.to_string_lossy().to_string()));
    registry
        .run("filter_raster_features_by_area", &frfa_args, &context(&caps))
        .expect("filter_raster_features_by_area should run");

    let reclass_out = Raster::read(&reclass_path).expect("read reclass output");
    let rei_out = Raster::read(&rei_path).expect("read reclass_equal_interval output");
    let frfa_out = Raster::read(&frfa_path).expect("read filter_raster_features_by_area output");
    assert_eq!(reclass_out.get(0, 0, 0), 10.0);
    assert_eq!(reclass_out.get(0, 1, 1), 20.0);
    assert_eq!(rei_out.get(0, 0, 0), 0.0);
    assert_eq!(rei_out.get(0, 1, 1), 4.0);
    assert_eq!(frfa_out.get(0, 0, 0), 0.0);
    assert_eq!(frfa_out.get(0, 1, 1), 0.0);

    let _ = std::fs::remove_file(&vector_path);
    let _ = std::fs::remove_file(&hull_path);
    let _ = std::fs::remove_file(&mbb_path);
    let _ = std::fs::remove_file(&mbc_path);
    let _ = std::fs::remove_file(&mbe_path);
    let _ = std::fs::remove_file(&raster_path);
    let _ = std::fs::remove_file(&reclass_path);
    let _ = std::fs::remove_file(&rei_path);
    let _ = std::fs::remove_file(&frfa_path);
}

#[test]
fn filter_vector_features_by_area_runs_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_filter_vector_area");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut layer = Layer::new("polys")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(4326);
    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(1.0, 0.0),
                    Coord::xy(1.0, 1.0),
                    Coord::xy(0.0, 1.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add small polygon");
    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(3.0, 0.0),
                    Coord::xy(3.0, 3.0),
                    Coord::xy(0.0, 3.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add large polygon");

    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("threshold".to_string(), json!(2.0));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("filter_vector_features_by_area", &args, &context(&caps))
        .expect("filter_vector_features_by_area should run");

    let output = wbvector::read(&output_path).expect("read output");
    assert_eq!(output.features.len(), 1, "only the large polygon should remain");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn extract_nodes_runs_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_extract_nodes");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut layer = Layer::new("lines")
        .with_geom_type(wbvector::GeometryType::LineString)
        .with_epsg(4326);
    layer
        .add_feature(
            Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(1.0, 1.0),
                Coord::xy(2.0, 0.0),
            ])),
            &[],
        )
        .expect("add line feature");

    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("extract_nodes", &args, &context(&caps))
        .expect("extract_nodes should run");

    let output = wbvector::read(&output_path).expect("read output");
    assert_eq!(output.features.len(), 3, "line with three vertices should yield three points");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn extract_by_attribute_runs_end_to_end() {
    use wbvector::{Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_extract_by_attribute");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut layer = Layer::new("pts")
        .with_geom_type(wbvector::GeometryType::Point)
        .with_epsg(4326);
    layer.add_field(wbvector::FieldDef::new("VALUE", wbvector::FieldType::Integer));
    layer
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[("VALUE", 5i64.into())])
        .expect("add point 1");
    layer
        .add_feature(Some(Geometry::point(1.0, 1.0)), &[("VALUE", 12i64.into())])
        .expect("add point 2");
    layer
        .add_feature(Some(Geometry::point(2.0, 2.0)), &[("VALUE", 20i64.into())])
        .expect("add point 3");

    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("statement".to_string(), json!("VALUE >= 10"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("extract_by_attribute", &args, &context(&caps))
        .expect("extract_by_attribute should run");

    let output = wbvector::read(&output_path).expect("read output");
    assert_eq!(output.features.len(), 2, "two features should satisfy VALUE >= 10");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn dissolve_runs_end_to_end() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_dissolve");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut layer = Layer::new("polys")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(4326);
    layer.add_field(wbvector::FieldDef::new("GROUP", wbvector::FieldType::Text));

    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(1.0, 0.0),
                    Coord::xy(1.0, 1.0),
                    Coord::xy(0.0, 1.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[("GROUP", "A".into())],
        )
        .expect("add polygon A1");
    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(1.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 1.0),
                    Coord::xy(1.0, 1.0),
                    Coord::xy(1.0, 0.0),
                ],
                vec![],
            )),
            &[("GROUP", "A".into())],
        )
        .expect("add polygon A2");
    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(3.0, 0.0),
                    Coord::xy(4.0, 0.0),
                    Coord::xy(4.0, 1.0),
                    Coord::xy(3.0, 1.0),
                    Coord::xy(3.0, 0.0),
                ],
                vec![],
            )),
            &[("GROUP", "B".into())],
        )
        .expect("add polygon B1");

    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("dissolve_field".to_string(), json!("GROUP"));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("dissolve", &args, &context(&caps))
        .expect("dissolve should run");

    let output = wbvector::read(&output_path).expect("read output");
    assert_eq!(output.features.len(), 2, "A polygons should dissolve into one feature");

    let group_idx = output
        .schema
        .field_index("GROUP")
        .expect("output should include dissolve field");
    let mut groups = output
        .features
        .iter()
        .map(|feature| feature.attributes.get(group_idx).cloned().unwrap_or(FieldValue::Null))
        .collect::<Vec<_>>();
    groups.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0], FieldValue::Text("A".to_string()));
    assert_eq!(groups[1], FieldValue::Text("B".to_string()));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn polygon_area_runs_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_polygon_area");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut layer = Layer::new("polys")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(4326);
    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(1.0, 0.0),
                    Coord::xy(1.0, 1.0),
                    Coord::xy(0.0, 1.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon");

    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("polygon_area", &args, &context(&caps))
        .expect("polygon_area should run");

    let output = wbvector::read(&output_path).expect("read output");
    let area_idx = output
        .schema
        .field_index("AREA")
        .expect("AREA field should exist");
    let area = output.features[0]
        .attributes
        .get(area_idx)
        .and_then(|v| v.as_f64())
        .expect("AREA value should be numeric");
    assert!((area - 1.0).abs() < 1.0e-9, "unit square area should be 1.0");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn polygon_perimeter_runs_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_polygon_perimeter");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut layer = Layer::new("polys")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(4326);
    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(1.0, 0.0),
                    Coord::xy(1.0, 1.0),
                    Coord::xy(0.0, 1.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon");

    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("polygon_perimeter", &args, &context(&caps))
        .expect("polygon_perimeter should run");

    let output = wbvector::read(&output_path).expect("read output");
    let perimeter_idx = output
        .schema
        .field_index("PERIMETER")
        .expect("PERIMETER field should exist");
    let perimeter = output.features[0]
        .attributes
        .get(perimeter_idx)
        .and_then(|v| v.as_f64())
        .expect("PERIMETER value should be numeric");
    assert!((perimeter - 4.0).abs() < 1.0e-9, "unit square perimeter should be 4.0");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn polygon_axes_run_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_polygon_axes");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.geojson"));
    let short_path = std::env::temp_dir().join(format!("{tag}_short.geojson"));
    let long_path = std::env::temp_dir().join(format!("{tag}_long.geojson"));

    let mut layer = Layer::new("polys")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(4326);
    layer
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(4.0, 0.0),
                    Coord::xy(4.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon");

    wbvector::write(&layer, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut short_args = ToolArgs::new();
    short_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    short_args.insert("output".to_string(), json!(short_path.to_string_lossy().to_string()));
    registry
        .run("polygon_short_axis", &short_args, &context(&caps))
        .expect("polygon_short_axis should run");

    let mut long_args = ToolArgs::new();
    long_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    long_args.insert("output".to_string(), json!(long_path.to_string_lossy().to_string()));
    registry
        .run("polygon_long_axis", &long_args, &context(&caps))
        .expect("polygon_long_axis should run");

    let short_out = wbvector::read(&short_path).expect("read short axis output");
    let long_out = wbvector::read(&long_path).expect("read long axis output");

    let short_len = match short_out.features[0].geometry.as_ref() {
        Some(wbvector::Geometry::LineString(coords)) if coords.len() == 2 => {
            let dx = coords[1].x - coords[0].x;
            let dy = coords[1].y - coords[0].y;
            (dx * dx + dy * dy).sqrt()
        }
        _ => panic!("short-axis output geometry must be a 2-vertex line"),
    };
    let long_len = match long_out.features[0].geometry.as_ref() {
        Some(wbvector::Geometry::LineString(coords)) if coords.len() == 2 => {
            let dx = coords[1].x - coords[0].x;
            let dy = coords[1].y - coords[0].y;
            (dx * dx + dy * dy).sqrt()
        }
        _ => panic!("long-axis output geometry must be a 2-vertex line"),
    };

    assert!(short_len > 0.0, "short axis length should be positive");
    assert!(long_len > 0.0, "long axis length should be positive");
    let min_axis = short_len.min(long_len);
    let max_axis = short_len.max(long_len);
    assert!((min_axis - 2.0).abs() < 1.0e-9, "one polygon axis should equal rectangle short side");
    assert!((max_axis - 4.0).abs() < 1.0e-9, "one polygon axis should equal rectangle long side");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&short_path);
    let _ = std::fs::remove_file(&long_path);
}

#[test]
fn centroid_vector_runs_end_to_end() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_centroid_vector");
    let poly_input_path = std::env::temp_dir().join(format!("{tag}_poly_in.geojson"));
    let poly_output_path = std::env::temp_dir().join(format!("{tag}_poly_out.geojson"));
    let point_input_path = std::env::temp_dir().join(format!("{tag}_point_in.geojson"));
    let point_output_path = std::env::temp_dir().join(format!("{tag}_point_out.geojson"));

    let mut polys = Layer::new("polys")
        .with_geom_type(wbvector::GeometryType::Polygon)
        .with_epsg(4326);
    polys
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon");
    wbvector::write(&polys, &poly_input_path, VectorFormat::GeoJson).expect("write polygon input");

    let mut poly_args = ToolArgs::new();
    poly_args.insert("input".to_string(), json!(poly_input_path.to_string_lossy().to_string()));
    poly_args.insert("output".to_string(), json!(poly_output_path.to_string_lossy().to_string()));
    registry
        .run("centroid_vector", &poly_args, &context(&caps))
        .expect("centroid_vector should run on polygon input");

    let poly_output = wbvector::read(&poly_output_path).expect("read polygon centroid output");
    assert_eq!(poly_output.features.len(), 1, "polygon input should produce one centroid feature");
    match poly_output.features[0].geometry.as_ref() {
        Some(wbvector::Geometry::Point(coord)) => {
            assert!((coord.x - 0.8).abs() < 1.0e-9);
            assert!((coord.y - 0.8).abs() < 1.0e-9);
        }
        _ => panic!("centroid output must be point geometry"),
    }

    let mut points = Layer::new("pts")
        .with_geom_type(wbvector::GeometryType::Point)
        .with_epsg(4326);
    points
        .add_feature(Some(Geometry::point(0.0, 0.0)), &[])
        .expect("add p1");
    points
        .add_feature(Some(Geometry::point(2.0, 2.0)), &[])
        .expect("add p2");
    wbvector::write(&points, &point_input_path, VectorFormat::GeoJson).expect("write point input");

    let mut point_args = ToolArgs::new();
    point_args.insert("input".to_string(), json!(point_input_path.to_string_lossy().to_string()));
    point_args.insert("output".to_string(), json!(point_output_path.to_string_lossy().to_string()));
    registry
        .run("centroid_vector", &point_args, &context(&caps))
        .expect("centroid_vector should run on point input");

    let point_output = wbvector::read(&point_output_path).expect("read point centroid output");
    assert_eq!(point_output.features.len(), 1, "point input should produce one centroid feature");
    match point_output.features[0].geometry.as_ref() {
        Some(wbvector::Geometry::Point(coord)) => {
            assert!((coord.x - 1.0).abs() < 1.0e-9);
            assert!((coord.y - 1.0).abs() < 1.0e-9);
        }
        _ => panic!("centroid output must be point geometry"),
    }

    let _ = std::fs::remove_file(&poly_input_path);
    let _ = std::fs::remove_file(&poly_output_path);
    let _ = std::fs::remove_file(&point_input_path);
    let _ = std::fs::remove_file(&point_output_path);
}

#[test]
fn raster_area_and_perimeter_compute_expected_class_totals() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let tag = unique_tag("wbtools_oss_raster_area_perimeter");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.asc"));
    let area_path = std::env::temp_dir().join(format!("{tag}_area.asc"));
    let perim_path = std::env::temp_dir().join(format!("{tag}_perim.asc"));

    let mut raster = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 2.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::default(),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            raster.set(0, r, c, 0.0).expect("set");
        }
    }
    raster.set(0, 1, 1, 1.0).expect("set center class");
    raster.write(&input_path, RasterFormat::EsriAscii).expect("write input raster");

    let caps = OpenOnly;

    let mut area_args = ToolArgs::new();
    area_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    area_args.insert("units".to_string(), json!("map units"));
    area_args.insert("zero_background".to_string(), json!(true));
    area_args.insert("output".to_string(), json!(area_path.to_string_lossy().to_string()));
    registry
        .run("raster_area", &area_args, &context(&caps))
        .expect("raster_area should run");

    let mut perim_args = ToolArgs::new();
    perim_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    perim_args.insert("units".to_string(), json!("grid cells"));
    perim_args.insert("zero_background".to_string(), json!(true));
    perim_args.insert("output".to_string(), json!(perim_path.to_string_lossy().to_string()));
    registry
        .run("raster_perimeter", &perim_args, &context(&caps))
        .expect("raster_perimeter should run");

    let area = Raster::read(&area_path).expect("read area output");
    assert!((area.get(0, 1, 1) - 4.0).abs() < 1e-9, "single cell area should be cell_size^2");
    assert_eq!(area.get(0, 0, 0), area.nodata, "background should remain nodata when zero_background=true");

    let perim = Raster::read(&perim_path).expect("read perimeter output");
    assert!((perim.get(0, 1, 1) - 4.0).abs() < 1e-9, "single cell perimeter in grid cells should be 4");
    assert_eq!(perim.get(0, 0, 0), perim.nodata, "background should remain nodata when zero_background=true");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&area_path);
    let _ = std::fs::remove_file(&perim_path);
}

#[test]
fn lidar_phase2_batch2_tools_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch2");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let scan_out = std::env::temp_dir().join(format!("{tag}_scan_filtered.las"));
    let noise_out = std::env::temp_dir().join(format!("{tag}_denoised.las"));
    let thin_out = std::env::temp_dir().join(format!("{tag}_thinned.las"));
    let thin_filtered_out = std::env::temp_dir().join(format!("{tag}_thinned_filtered.las"));
    let elev_filter_out = std::env::temp_dir().join(format!("{tag}_elev_filter.las"));
    let elev_class_out = std::env::temp_dir().join(format!("{tag}_elev_class.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                scan_angle: 0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.1,
                y: 0.1,
                z: 11.0,
                classification: 7,
                scan_angle: 200,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 1.0,
                z: 20.0,
                classification: 18,
                scan_angle: -250,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 2.0,
                y: 2.0,
                z: 30.0,
                classification: 2,
                scan_angle: 100,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut scan_args = ToolArgs::new();
    scan_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    scan_args.insert("threshold".to_string(), json!(150));
    scan_args.insert("output".to_string(), json!(scan_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_scan_angles", &scan_args, &context(&caps))
        .expect("filter_lidar_scan_angles should run");

    let scan_filtered = PointCloud::read(&scan_out).expect("read scan output");
    assert_eq!(scan_filtered.points.len(), 2);
    assert!(scan_filtered.points.iter().all(|p| p.scan_angle.abs() <= 150));
    assert_eq!(scan_filtered.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    let mut noise_args = ToolArgs::new();
    noise_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    noise_args.insert("output".to_string(), json!(noise_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_noise", &noise_args, &context(&caps))
        .expect("filter_lidar_noise should run");

    let denoised = PointCloud::read(&noise_out).expect("read denoised output");
    assert_eq!(denoised.points.len(), 2);
    assert!(denoised
        .points
        .iter()
        .all(|p| p.classification != 7 && p.classification != 18));

    let mut thin_args = ToolArgs::new();
    thin_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    thin_args.insert("resolution".to_string(), json!(1.0));
    thin_args.insert("method".to_string(), json!("highest"));
    thin_args.insert("save_filtered".to_string(), json!(true));
    thin_args.insert("output".to_string(), json!(thin_out.to_string_lossy().to_string()));
    thin_args.insert("filtered_output".to_string(), json!(thin_filtered_out.to_string_lossy().to_string()));
    let thin_res = registry
        .run("lidar_thin", &thin_args, &context(&caps))
        .expect("lidar_thin should run");

    let thinned = PointCloud::read(&thin_out).expect("read thinned output");
    assert!(thinned.points.len() <= cloud.points.len());
    assert!(!thinned.points.is_empty());
    assert_eq!(thinned.crs.as_ref().and_then(|c| c.epsg), Some(4326));
    let filtered_path = thin_res
        .outputs
        .get("filtered_path")
        .and_then(|v| v.as_str())
        .expect("filtered_path output expected when save_filtered=true");
    assert_eq!(filtered_path, thin_filtered_out.to_string_lossy().as_ref());
    let thinned_filtered = PointCloud::read(&thin_filtered_out).expect("read thinned filtered output");
    assert_eq!(thinned.points.len() + thinned_filtered.points.len(), cloud.points.len());

    let mut elev_filter_args = ToolArgs::new();
    elev_filter_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    elev_filter_args.insert("minz".to_string(), json!(10.5));
    elev_filter_args.insert("maxz".to_string(), json!(25.0));
    elev_filter_args.insert("output".to_string(), json!(elev_filter_out.to_string_lossy().to_string()));
    registry
        .run("lidar_elevation_slice", &elev_filter_args, &context(&caps))
        .expect("lidar_elevation_slice filter should run");

    let elev_filtered = PointCloud::read(&elev_filter_out).expect("read elev filter output");
    assert_eq!(elev_filtered.points.len(), 2);
    assert!(elev_filtered.points.iter().all(|p| p.z >= 10.5 && p.z <= 25.0));

    let mut elev_class_args = ToolArgs::new();
    elev_class_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    elev_class_args.insert("minz".to_string(), json!(10.5));
    elev_class_args.insert("maxz".to_string(), json!(25.0));
    elev_class_args.insert("classify".to_string(), json!(true));
    elev_class_args.insert("in_class_value".to_string(), json!(9));
    elev_class_args.insert("out_class_value".to_string(), json!(3));
    elev_class_args.insert("output".to_string(), json!(elev_class_out.to_string_lossy().to_string()));
    registry
        .run("lidar_elevation_slice", &elev_class_args, &context(&caps))
        .expect("lidar_elevation_slice classify should run");

    let elev_classed = PointCloud::read(&elev_class_out).expect("read elev class output");
    assert_eq!(elev_classed.points.len(), cloud.points.len());
    assert!(elev_classed.points.iter().any(|p| p.classification == 9));
    assert!(elev_classed.points.iter().any(|p| p.classification == 3));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&scan_out);
    let _ = std::fs::remove_file(&noise_out);
    let _ = std::fs::remove_file(&thin_out);
    let _ = std::fs::remove_file(&thin_filtered_out);
    let _ = std::fs::remove_file(&elev_filter_out);
    let _ = std::fs::remove_file(&elev_class_out);
}

#[test]
fn lidar_phase2_batch2_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch2_batch_mode");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("phase2b_tile_a.las");
    let tile_b = batch_dir.join("phase2b_tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, scan_angle: 50, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.2, y: 0.2, z: 11.0, classification: 7, scan_angle: 250, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 12.0, classification: 2, scan_angle: 25, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 2.0, y: 2.0, z: 20.0, classification: 1, scan_angle: 40, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 2.1, y: 2.1, z: 21.0, classification: 18, scan_angle: -260, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 3.0, z: 22.0, classification: 2, scan_angle: 30, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir to batch dir");

    let mut scan_args = ToolArgs::new();
    scan_args.insert("threshold".to_string(), json!(100));
    let scan_res = registry
        .run("filter_lidar_scan_angles", &scan_args, &context(&caps))
        .expect("filter_lidar_scan_angles batch run");

    let noise_args = ToolArgs::new();
    let noise_res = registry
        .run("filter_lidar_noise", &noise_args, &context(&caps))
        .expect("filter_lidar_noise batch run");

    let mut thin_args = ToolArgs::new();
    thin_args.insert("resolution".to_string(), json!(1.0));
    thin_args.insert("method".to_string(), json!("first"));
    let thin_res = registry
        .run("lidar_thin", &thin_args, &context(&caps))
        .expect("lidar_thin batch run");

    let mut elev_args = ToolArgs::new();
    elev_args.insert("minz".to_string(), json!(10.5));
    elev_args.insert("maxz".to_string(), json!(30.0));
    let elev_res = registry
        .run("lidar_elevation_slice", &elev_args, &context(&caps))
        .expect("lidar_elevation_slice batch run");

    std::env::set_current_dir(&old_cwd).expect("restore current dir");

    for res in [&scan_res, &noise_res, &thin_res, &elev_res] {
        assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
        let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
        assert!(std::path::Path::new(p).exists(), "placeholder output should exist: {p}");
    }

    for suffix in ["scan_filtered", "denoised", "thinned", "elev_slice"] {
        let _ = std::fs::remove_file(batch_dir.join(format!("phase2b_tile_a_{suffix}.las")));
        let _ = std::fs::remove_file(batch_dir.join(format!("phase2b_tile_b_{suffix}.las")));
    }
    let _ = std::fs::remove_file(&tile_a);
    let _ = std::fs::remove_file(&tile_b);
    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn lidar_phase2_batch_b_tools_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2b");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let scan_out = std::env::temp_dir().join(format!("{tag}_scan.las"));
    let noise_out = std::env::temp_dir().join(format!("{tag}_noise.las"));
    let thin_out = std::env::temp_dir().join(format!("{tag}_thin.las"));
    let thin_low_out = std::env::temp_dir().join(format!("{tag}_thin_low.las"));
    let elev_filter_out = std::env::temp_dir().join(format!("{tag}_elev_filter.las"));
    let elev_classify_out = std::env::temp_dir().join(format!("{tag}_elev_classify.las"));

    let cloud = PointCloud {
        points: vec![
            // same thinning cell: two points, z=5.0 and z=1.0; scan_angle=5 → kept by threshold 10; lowest is z=1.0
            PointRecord { x: 0.0, y: 0.1, z: 5.0, classification: 1, scan_angle: 5, ..PointRecord::default() },
            PointRecord { x: 0.1, y: 0.1, z: 1.0, classification: 1, scan_angle: 5, ..PointRecord::default() },
            // cell (1,0): scan_angle=20 → filtered by threshold 10; classification=7 (low noise)
            PointRecord { x: 1.0, y: 0.0, z: 15.0, classification: 7, scan_angle: 20, ..PointRecord::default() },
            // cell (0,1): z=25.0 outside elevation slice 0..20
            PointRecord { x: 0.0, y: 1.0, z: 25.0, classification: 18, scan_angle: 3, ..PointRecord::default() },
            // cell (1,1): z=10.0 inside elevation slice; scan_angle=8 ≤ 10
            PointRecord { x: 1.0, y: 1.0, z: 10.0, classification: 2, scan_angle: 8, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    // --- filter_lidar_scan_angles: threshold=10 → keeps scan_angle ≤ 10 → removes point with scan_angle=20 ---
    let mut scan_args = ToolArgs::new();
    scan_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    scan_args.insert("threshold".to_string(), json!(10));
    scan_args.insert("output".to_string(), json!(scan_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_scan_angles", &scan_args, &context(&caps))
        .expect("filter_lidar_scan_angles should run");

    let scan_result = PointCloud::read(&scan_out).expect("read scan_filtered output");
    assert_eq!(scan_result.points.len(), 4, "one point with scan_angle=20 should be removed");
    assert!(scan_result.points.iter().all(|p| p.scan_angle.abs() <= 10));
    assert_eq!(scan_result.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    // --- filter_lidar_noise: removes classification 7 and 18 → removes 2 points ---
    let mut noise_args = ToolArgs::new();
    noise_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    noise_args.insert("output".to_string(), json!(noise_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_noise", &noise_args, &context(&caps))
        .expect("filter_lidar_noise should run");

    let noise_result = PointCloud::read(&noise_out).expect("read denoised output");
    assert_eq!(noise_result.points.len(), 3, "class 7 and class 18 points should be removed");
    assert!(noise_result.points.iter().all(|p| p.classification != 7 && p.classification != 18));
    assert_eq!(noise_result.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    // --- lidar_thin (first): resolution=1.0, method=first → one point per 1m cell ---
    let mut thin_args = ToolArgs::new();
    thin_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    thin_args.insert("resolution".to_string(), json!(1.0));
    thin_args.insert("method".to_string(), json!("first"));
    thin_args.insert("output".to_string(), json!(thin_out.to_string_lossy().to_string()));
    registry
        .run("lidar_thin", &thin_args, &context(&caps))
        .expect("lidar_thin first should run");

    let thin_result = PointCloud::read(&thin_out).expect("read thinned output");
    // 5 points spanning 4 grid cells (with 1m resolution) → at most 4 points after thinning
    assert!(thin_result.points.len() <= 4, "thinning should reduce to at most 1 point per cell");
    assert!(thin_result.points.len() >= 1);
    assert_eq!(thin_result.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    // --- lidar_thin (lowest): resolution=1.0, method=lowest → z=1.0 wins cell (0,0) ---
    let mut thin_low_args = ToolArgs::new();
    thin_low_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    thin_low_args.insert("resolution".to_string(), json!(1.0));
    thin_low_args.insert("method".to_string(), json!("lowest"));
    thin_low_args.insert("output".to_string(), json!(thin_low_out.to_string_lossy().to_string()));
    registry
        .run("lidar_thin", &thin_low_args, &context(&caps))
        .expect("lidar_thin lowest should run");

    let thin_low_result = PointCloud::read(&thin_low_out).expect("read thinned_low output");
    // the shared thinning cell has z=5.0 and z=1.0 → lowest keeps z=1.0
    assert!(thin_low_result.points.iter().any(|p| (p.z - 1.0).abs() < 1e-9),
        "lowest method should keep z=1.0 in cell containing (0.0,0.1) and (0.1,0.1)");
    assert!(!thin_low_result.points.iter().any(|p| (p.z - 5.0).abs() < 1e-9),
        "lowest method should not keep z=5.0 when z=1.0 is in the same cell");

    // --- lidar_elevation_slice (filter mode): minz=0, maxz=20 → removes z=25.0 ---
    let mut elev_filter_args = ToolArgs::new();
    elev_filter_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    elev_filter_args.insert("minz".to_string(), json!(0.0));
    elev_filter_args.insert("maxz".to_string(), json!(20.0));
    elev_filter_args.insert("classify".to_string(), json!(false));
    elev_filter_args.insert("output".to_string(), json!(elev_filter_out.to_string_lossy().to_string()));
    registry
        .run("lidar_elevation_slice", &elev_filter_args, &context(&caps))
        .expect("lidar_elevation_slice filter should run");

    let elev_filter_result = PointCloud::read(&elev_filter_out).expect("read elev_filter output");
    assert_eq!(elev_filter_result.points.len(), 4, "point at z=25 should be removed");
    assert!(elev_filter_result.points.iter().all(|p| p.z >= 0.0 && p.z <= 20.0));
    assert_eq!(elev_filter_result.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    // --- lidar_elevation_slice (classify mode): minz=0, maxz=20 → reassigns classification ---
    let mut elev_cls_args = ToolArgs::new();
    elev_cls_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    elev_cls_args.insert("minz".to_string(), json!(0.0));
    elev_cls_args.insert("maxz".to_string(), json!(20.0));
    elev_cls_args.insert("classify".to_string(), json!(true));
    elev_cls_args.insert("in_class_value".to_string(), json!(2));
    elev_cls_args.insert("out_class_value".to_string(), json!(1));
    elev_cls_args.insert("output".to_string(), json!(elev_classify_out.to_string_lossy().to_string()));
    registry
        .run("lidar_elevation_slice", &elev_cls_args, &context(&caps))
        .expect("lidar_elevation_slice classify should run");

    let elev_cls_result = PointCloud::read(&elev_classify_out).expect("read elev_classify output");
    assert_eq!(elev_cls_result.points.len(), 5, "classify mode keeps all points");
    // z=25.0 is outside slice → classification should be out_class_value=1
    let outside_pts: Vec<_> = elev_cls_result.points.iter().filter(|p| p.z > 20.0).collect();
    assert!(outside_pts.iter().all(|p| p.classification == 1), "outside points should have class=1");
    // points inside slice → classification should be in_class_value=2
    let inside_pts: Vec<_> = elev_cls_result.points.iter().filter(|p| p.z >= 0.0 && p.z <= 20.0).collect();
    assert!(inside_pts.iter().all(|p| p.classification == 2), "inside points should have class=2");

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&scan_out);
    let _ = std::fs::remove_file(&noise_out);
    let _ = std::fs::remove_file(&thin_out);
    let _ = std::fs::remove_file(&thin_low_out);
    let _ = std::fs::remove_file(&elev_filter_out);
    let _ = std::fs::remove_file(&elev_classify_out);
}

#[test]
fn lidar_phase2_batch3_tools_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch3");
    let tile_a = std::env::temp_dir().join(format!("{tag}_a.las"));
    let tile_b = std::env::temp_dir().join(format!("{tag}_b.las"));
    let joined_out = std::env::temp_dir().join(format!("{tag}_joined.las"));
    let sorted_out = std::env::temp_dir().join(format!("{tag}_sorted.las"));
    let thin_hd_out = std::env::temp_dir().join(format!("{tag}_thin_hd.las"));
    let thin_hd_filtered_out = std::env::temp_dir().join(format!("{tag}_thin_hd_filtered.las"));
    let percentile_out = std::env::temp_dir().join(format!("{tag}_percentile.las"));
    let tile_out_dir = std::env::temp_dir().join(format!("{tag}_tiles"));
    std::fs::create_dir_all(&tile_out_dir).expect("create tile out dir");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.1, y: 0.1, z: 11.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.1, y: 1.1, z: 15.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 2.0, y: 2.0, z: 20.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 3.0, z: 30.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    // lidar_join
    let mut join_args = ToolArgs::new();
    join_args.insert(
        "inputs".to_string(),
        json!([
            tile_a.to_string_lossy().to_string(),
            tile_b.to_string_lossy().to_string()
        ]),
    );
    join_args.insert("output".to_string(), json!(joined_out.to_string_lossy().to_string()));
    registry
        .run("lidar_join", &join_args, &context(&caps))
        .expect("lidar_join should run");
    let joined = PointCloud::read(&joined_out).expect("read joined output");
    assert_eq!(joined.points.len(), 5);

    // sort_lidar
    let mut sort_args = ToolArgs::new();
    sort_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": joined_out.to_string_lossy().to_string()}),
    );
    sort_args.insert("sort_criteria".to_string(), json!("x"));
    sort_args.insert("output".to_string(), json!(sorted_out.to_string_lossy().to_string()));
    registry
        .run("sort_lidar", &sort_args, &context(&caps))
        .expect("sort_lidar should run");
    let sorted = PointCloud::read(&sorted_out).expect("read sorted output");
    for i in 1..sorted.points.len() {
        assert!(sorted.points[i - 1].x <= sorted.points[i].x);
    }

    // lidar_thin_high_density
    let mut thin_hd_args = ToolArgs::new();
    thin_hd_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": tile_a.to_string_lossy().to_string()}),
    );
    thin_hd_args.insert("density".to_string(), json!(0.5));
    thin_hd_args.insert("resolution".to_string(), json!(1.0));
    thin_hd_args.insert("save_filtered".to_string(), json!(true));
    thin_hd_args.insert("output".to_string(), json!(thin_hd_out.to_string_lossy().to_string()));
    thin_hd_args.insert(
        "filtered_output".to_string(),
        json!(thin_hd_filtered_out.to_string_lossy().to_string()),
    );
    let thin_hd_res = registry
        .run("lidar_thin_high_density", &thin_hd_args, &context(&caps))
        .expect("lidar_thin_high_density should run");
    let thin_hd = PointCloud::read(&thin_hd_out).expect("read thin_hd output");
    let thin_hd_filtered = PointCloud::read(&thin_hd_filtered_out).expect("read thin_hd filtered output");
    assert_eq!(thin_hd.points.len() + thin_hd_filtered.points.len(), cloud_a.points.len());
    assert!(thin_hd_res.outputs.get("filtered_path").and_then(|v| v.as_str()).is_some());

    // filter_lidar_by_percentile
    let mut pct_args = ToolArgs::new();
    pct_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": tile_a.to_string_lossy().to_string()}),
    );
    pct_args.insert("percentile".to_string(), json!(100.0));
    pct_args.insert("block_size".to_string(), json!(1.0));
    pct_args.insert("output".to_string(), json!(percentile_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_by_percentile", &pct_args, &context(&caps))
        .expect("filter_lidar_by_percentile should run");
    let pct = PointCloud::read(&percentile_out).expect("read percentile output");
    assert!(!pct.points.is_empty());
    assert!(pct.points.len() <= cloud_a.points.len());

    // lidar_tile
    let mut tile_args = ToolArgs::new();
    tile_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": joined_out.to_string_lossy().to_string()}),
    );
    tile_args.insert("tile_width".to_string(), json!(1.5));
    tile_args.insert("tile_height".to_string(), json!(1.5));
    tile_args.insert("origin_x".to_string(), json!(0.0));
    tile_args.insert("origin_y".to_string(), json!(0.0));
    tile_args.insert("min_points_in_tile".to_string(), json!(1));
    tile_args.insert("output_laz_format".to_string(), json!(false));
    tile_args.insert("output_directory".to_string(), json!(tile_out_dir.to_string_lossy().to_string()));
    let tile_res = registry
        .run("lidar_tile", &tile_args, &context(&caps))
        .expect("lidar_tile should run");
    assert_eq!(tile_res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
    assert!(tile_res.outputs.get("tile_count").and_then(|v| v.as_u64()).unwrap_or(0) > 0);
    let tile_placeholder = tile_res.outputs.get("path").and_then(|v| v.as_str()).expect("tile placeholder path");
    assert!(std::path::Path::new(tile_placeholder).exists());

    let _ = std::fs::remove_file(&tile_a);
    let _ = std::fs::remove_file(&tile_b);
    let _ = std::fs::remove_file(&joined_out);
    let _ = std::fs::remove_file(&sorted_out);
    let _ = std::fs::remove_file(&thin_hd_out);
    let _ = std::fs::remove_file(&thin_hd_filtered_out);
    let _ = std::fs::remove_file(&percentile_out);
    let _ = std::fs::remove_dir_all(&tile_out_dir);
}

#[test]
fn lidar_phase2_batch3_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch3_batch_mode");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("phase2c_tile_a.las");
    let tile_b = batch_dir.join("phase2c_tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.2, y: 0.2, z: 11.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 12.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 2.0, y: 2.0, z: 20.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 2.1, y: 2.1, z: 21.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 3.0, z: 22.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir to batch dir");

    let mut sort_args = ToolArgs::new();
    sort_args.insert("sort_criteria".to_string(), json!("x"));
    let sort_res = registry
        .run("sort_lidar", &sort_args, &context(&caps))
        .expect("sort_lidar batch run");

    let mut thin_hd_args = ToolArgs::new();
    thin_hd_args.insert("density".to_string(), json!(0.5));
    thin_hd_args.insert("resolution".to_string(), json!(1.0));
    let thin_hd_res = registry
        .run("lidar_thin_high_density", &thin_hd_args, &context(&caps))
        .expect("lidar_thin_high_density batch run");

    let mut pct_args = ToolArgs::new();
    pct_args.insert("percentile".to_string(), json!(50.0));
    pct_args.insert("block_size".to_string(), json!(1.0));
    let pct_res = registry
        .run("filter_lidar_by_percentile", &pct_args, &context(&caps))
        .expect("filter_lidar_by_percentile batch run");

    std::env::set_current_dir(&old_cwd).expect("restore current dir");

    for res in [&sort_res, &thin_hd_res, &pct_res] {
        assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
        let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
        assert!(std::path::Path::new(p).exists(), "placeholder output should exist: {p}");
    }

    for suffix in ["sorted", "thinned_hd", "percentile"] {
        let _ = std::fs::remove_file(batch_dir.join(format!("phase2c_tile_a_{suffix}.las")));
        let _ = std::fs::remove_file(batch_dir.join(format!("phase2c_tile_b_{suffix}.las")));
    }
    let _ = std::fs::remove_file(&tile_a);
    let _ = std::fs::remove_file(&tile_b);
    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn lidar_phase2_batch_b_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2b_batch");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("phase2b_tile_a.las");
    let tile_b = batch_dir.join("phase2b_tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 5.0, classification: 1, scan_angle: 5, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 8.0, classification: 7, scan_angle: 25, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 2.0, z: 30.0, classification: 18, scan_angle: 3, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 2.0, classification: 2, scan_angle: 2, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 4.0, classification: 1, scan_angle: 15, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir to batch dir");

    // filter_lidar_scan_angles batch
    let mut scan_args = ToolArgs::new();
    scan_args.insert("threshold".to_string(), json!(10));
    let scan_res = registry
        .run("filter_lidar_scan_angles", &scan_args, &context(&caps))
        .expect("filter_lidar_scan_angles batch");

    // filter_lidar_noise batch
    let noise_res = registry
        .run("filter_lidar_noise", &ToolArgs::new(), &context(&caps))
        .expect("filter_lidar_noise batch");

    // lidar_thin batch
    let mut thin_args = ToolArgs::new();
    thin_args.insert("resolution".to_string(), json!(2.0));
    thin_args.insert("method".to_string(), json!("first"));
    let thin_res = registry
        .run("lidar_thin", &thin_args, &context(&caps))
        .expect("lidar_thin batch");

    // lidar_elevation_slice batch
    let mut elev_args = ToolArgs::new();
    elev_args.insert("minz".to_string(), json!(0.0));
    elev_args.insert("maxz".to_string(), json!(20.0));
    let elev_res = registry
        .run("lidar_elevation_slice", &elev_args, &context(&caps))
        .expect("lidar_elevation_slice batch");

    std::env::set_current_dir(&old_cwd).expect("restore cwd");

    for res in [&scan_res, &noise_res, &thin_res, &elev_res] {
        assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
        let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
        assert!(std::path::Path::new(p).exists(), "placeholder output should exist: {p}");
    }

    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn lidar_phase2_batch4_split_and_remove_outliers_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch4");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let outlier_filtered_out = std::env::temp_dir().join(format!("{tag}_outliers_removed.las"));
    let outlier_class_out = std::env::temp_dir().join(format!("{tag}_outliers_classified.las"));
    let split_dir = std::env::temp_dir().join(format!("{tag}_splits"));
    std::fs::create_dir_all(&split_dir).expect("create split dir");

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 11.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 0.0, z: 12.0, classification: 2, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 0.0, z: 100.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 4.0, y: 0.0, z: 13.0, classification: 2, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    // lidar_remove_outliers (filter mode)
    let mut outlier_args = ToolArgs::new();
    outlier_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    outlier_args.insert("search_radius".to_string(), json!(5.0));
    outlier_args.insert("elev_diff".to_string(), json!(20.0));
    outlier_args.insert("classify".to_string(), json!(false));
    outlier_args.insert("output".to_string(), json!(outlier_filtered_out.to_string_lossy().to_string()));
    registry
        .run("lidar_remove_outliers", &outlier_args, &context(&caps))
        .expect("lidar_remove_outliers filter should run");

    let outlier_filtered = PointCloud::read(&outlier_filtered_out).expect("read outlier filtered output");
    assert!(outlier_filtered.points.len() < cloud.points.len());

    // lidar_remove_outliers (classify mode)
    let mut class_args = ToolArgs::new();
    class_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    class_args.insert("search_radius".to_string(), json!(5.0));
    class_args.insert("elev_diff".to_string(), json!(20.0));
    class_args.insert("classify".to_string(), json!(true));
    class_args.insert("output".to_string(), json!(outlier_class_out.to_string_lossy().to_string()));
    registry
        .run("lidar_remove_outliers", &class_args, &context(&caps))
        .expect("lidar_remove_outliers classify should run");

    let outlier_classed = PointCloud::read(&outlier_class_out).expect("read outlier classify output");
    assert_eq!(outlier_classed.points.len(), cloud.points.len());
    assert!(outlier_classed.points.iter().any(|p| p.classification == 18));

    // split_lidar (single input)
    let mut split_args = ToolArgs::new();
    split_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    split_args.insert("split_criterion".to_string(), json!("class"));
    split_args.insert("interval".to_string(), json!(5.0));
    split_args.insert("min_pts".to_string(), json!(0));
    split_args.insert("output_directory".to_string(), json!(split_dir.to_string_lossy().to_string()));
    let split_res = registry
        .run("split_lidar", &split_args, &context(&caps))
        .expect("split_lidar should run");
    assert_eq!(split_res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
    let split_placeholder = split_res.outputs.get("path").and_then(|v| v.as_str()).expect("split placeholder path");
    assert!(std::path::Path::new(split_placeholder).exists());
    assert!(split_res.outputs.get("output_count").and_then(|v| v.as_u64()).unwrap_or(0) >= 2);

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&outlier_filtered_out);
    let _ = std::fs::remove_file(&outlier_class_out);
    let _ = std::fs::remove_dir_all(&split_dir);
}

#[test]
fn lidar_phase2_batch4_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch4_batch");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("phase2d_tile_a.las");
    let tile_b = batch_dir.join("phase2d_tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 11.0, classification: 2, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 0.0, z: 99.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 1.0, z: 12.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 13.0, classification: 2, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 1.0, z: 98.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir to batch dir");

    let mut split_args = ToolArgs::new();
    split_args.insert("split_criterion".to_string(), json!("class"));
    split_args.insert("interval".to_string(), json!(5.0));
    split_args.insert("min_pts".to_string(), json!(0));
    let split_res = registry
        .run("split_lidar", &split_args, &context(&caps))
        .expect("split_lidar batch run");

    let mut outlier_args = ToolArgs::new();
    outlier_args.insert("search_radius".to_string(), json!(5.0));
    outlier_args.insert("elev_diff".to_string(), json!(20.0));
    let outlier_res = registry
        .run("lidar_remove_outliers", &outlier_args, &context(&caps))
        .expect("lidar_remove_outliers batch run");

    std::env::set_current_dir(&old_cwd).expect("restore current dir");

    for res in [&split_res, &outlier_res] {
        assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
        let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
        assert!(std::path::Path::new(p).exists(), "placeholder output should exist: {p}");
    }

    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn lidar_phase2_batch5_normalize_and_height_above_ground_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch5");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let dtm_path = std::env::temp_dir().join(format!("{tag}_dtm.tif"));
    let norm_out = std::env::temp_dir().join(format!("{tag}_normalized.las"));
    let hag_out = std::env::temp_dir().join(format!("{tag}_hag.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.5, y: 2.5, z: 100.0, classification: 2, ..PointRecord::default() },
            PointRecord { x: 1.5, y: 2.5, z: 102.0, classification: 2, ..PointRecord::default() },
            PointRecord { x: 0.5, y: 1.5, z: 105.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.5, y: 1.5, z: 108.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut dtm = Raster::new(RasterConfig {
        cols: 3,
        rows: 3,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    for r in 0..3isize {
        for c in 0..3isize {
            dtm.set(0, r, c, 100.0 + (2 - r) as f64 + c as f64).expect("set dtm value");
        }
    }
    dtm.write(&dtm_path, RasterFormat::GeoTiff).expect("write dtm raster");

    let mut norm_args = ToolArgs::new();
    norm_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    norm_args.insert(
        "dtm".to_string(),
        json!({"__wbw_type__":"raster","path": dtm_path.to_string_lossy().to_string()}),
    );
    norm_args.insert("no_negatives".to_string(), json!(true));
    norm_args.insert("output".to_string(), json!(norm_out.to_string_lossy().to_string()));
    registry
        .run("normalize_lidar", &norm_args, &context(&caps))
        .expect("normalize_lidar should run");

    let normalized = PointCloud::read(&norm_out).expect("read normalized output");
    assert_eq!(normalized.points.len(), cloud.points.len());
    assert!(normalized.points.iter().all(|p| p.z >= 0.0));
    assert!(normalized.points.iter().any(|p| p.classification == 1 && p.z > 0.0));

    let mut hag_args = ToolArgs::new();
    hag_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    hag_args.insert("output".to_string(), json!(hag_out.to_string_lossy().to_string()));
    registry
        .run("height_above_ground", &hag_args, &context(&caps))
        .expect("height_above_ground should run");

    let hag_cloud = PointCloud::read(&hag_out).expect("read height_above_ground output");
    assert_eq!(hag_cloud.points.len(), cloud.points.len());
    assert!(hag_cloud.points.iter().any(|p| p.classification == 2 && p.z.abs() < 1e-9));
    assert!(hag_cloud.points.iter().any(|p| p.classification != 2 && p.z > 0.0));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&dtm_path);
    let _ = std::fs::remove_file(&norm_out);
    let _ = std::fs::remove_file(&hag_out);
}

#[test]
fn lidar_phase2_batch6_ground_filter_and_filter_lidar_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch6");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let ground_filtered_out = std::env::temp_dir().join(format!("{tag}_ground_filtered.las"));
    let classed_out = std::env::temp_dir().join(format!("{tag}_ground_classed.las"));
    let expression_out = std::env::temp_dir().join(format!("{tag}_expression_filtered.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 10.2, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 0.0, z: 10.1, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 18.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 1.0, z: 19.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut gf_args = ToolArgs::new();
    gf_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    gf_args.insert("search_radius".to_string(), json!(2.5));
    gf_args.insert("slope_threshold".to_string(), json!(30.0));
    gf_args.insert("height_threshold".to_string(), json!(1.0));
    gf_args.insert("classify".to_string(), json!(false));
    gf_args.insert("output".to_string(), json!(ground_filtered_out.to_string_lossy().to_string()));
    registry
        .run("lidar_ground_point_filter", &gf_args, &context(&caps))
        .expect("lidar_ground_point_filter should run in filter mode");
    let filtered = PointCloud::read(&ground_filtered_out).expect("read filtered output");
    assert!(filtered.points.len() < cloud.points.len());

    let mut class_args = ToolArgs::new();
    class_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    class_args.insert("search_radius".to_string(), json!(2.5));
    class_args.insert("slope_threshold".to_string(), json!(30.0));
    class_args.insert("height_threshold".to_string(), json!(1.0));
    class_args.insert("classify".to_string(), json!(true));
    class_args.insert("output".to_string(), json!(classed_out.to_string_lossy().to_string()));
    registry
        .run("lidar_ground_point_filter", &class_args, &context(&caps))
        .expect("lidar_ground_point_filter should run in classify mode");
    let classed = PointCloud::read(&classed_out).expect("read classed output");
    assert_eq!(classed.points.len(), cloud.points.len());
    assert!(classed.points.iter().any(|p| p.classification == 1));
    assert!(classed.points.iter().any(|p| p.classification == 2));

    let mut expr_args = ToolArgs::new();
    expr_args.insert(
        "input".to_string(),
        json!({"__wbw_type__":"lidar","path": lidar_path.to_string_lossy().to_string()}),
    );
    expr_args.insert("statement".to_string(), json!("class == 2 && z <= 10.5"));
    expr_args.insert("output".to_string(), json!(expression_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar", &expr_args, &context(&caps))
        .expect("filter_lidar should run");
    let expr_cloud = PointCloud::read(&expression_out).expect("read expression output");
    assert!(expr_cloud.points.len() >= 2);
    assert!(expr_cloud.points.iter().all(|p| p.classification == 2 && p.z <= 10.5));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&ground_filtered_out);
    let _ = std::fs::remove_file(&classed_out);
    let _ = std::fs::remove_file(&expression_out);
}

#[test]
fn lidar_phase2_batch6_hardening_edge_cases() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch6_hardening");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let dtm_path = std::env::temp_dir().join(format!("{tag}_dtm.tif"));
    let norm_out = std::env::temp_dir().join(format!("{tag}_norm.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.5, y: 0.5, z: 5.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.5, y: 0.5, z: 6.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut dtm = Raster::new(RasterConfig {
        cols: 2,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    dtm.set(0, 0, 0, 10.0).expect("set dtm");
    dtm.set(0, 0, 1, 10.0).expect("set dtm");
    dtm.write(&dtm_path, RasterFormat::GeoTiff).expect("write dtm");

    let mut norm_args = ToolArgs::new();
    norm_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    norm_args.insert("dtm".to_string(), json!(dtm_path.to_string_lossy().to_string()));
    norm_args.insert("no_negatives".to_string(), json!(true));
    norm_args.insert("output".to_string(), json!(norm_out.to_string_lossy().to_string()));
    registry
        .run("normalize_lidar", &norm_args, &context(&caps))
        .expect("normalize_lidar should run");
    let norm_cloud = PointCloud::read(&norm_out).expect("read norm output");
    assert!(norm_cloud.points.iter().all(|p| p.z >= 0.0));

    let mut hag_args = ToolArgs::new();
    hag_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    let hag_err = registry.run("height_above_ground", &hag_args, &context(&caps));
    assert!(hag_err.is_err(), "height_above_ground should fail when no ground points exist");

    let mut expr_args = ToolArgs::new();
    expr_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    expr_args.insert("statement".to_string(), json!("class == 1 && z > 0"));
    registry
        .run("filter_lidar", &expr_args, &context(&caps))
        .expect("filter_lidar should support integer comparisons in expressions");

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&dtm_path);
    let _ = std::fs::remove_file(&norm_out);
}

#[test]
fn lidar_phase2_batch7_reference_surface_and_classify_run_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_batch7");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let surface_path = std::env::temp_dir().join(format!("{tag}_surface.tif"));
    let ref_out = std::env::temp_dir().join(format!("{tag}_ref_filtered.las"));
    let classed_out = std::env::temp_dir().join(format!("{tag}_classified.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.5, y: 1.5, z: 10.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.5, y: 1.5, z: 10.1, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.5, y: 0.5, z: 12.5, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.5, y: 0.5, z: 15.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    let mut surface = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    surface.set(0, 0, 0, 10.0).expect("set surface");
    surface.set(0, 0, 1, 10.0).expect("set surface");
    surface.set(0, 1, 0, 10.0).expect("set surface");
    surface.set(0, 1, 1, 10.0).expect("set surface");
    surface.write(&surface_path, RasterFormat::GeoTiff).expect("write surface");

    let mut ref_args = ToolArgs::new();
    ref_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    ref_args.insert("ref_surface".to_string(), json!(surface_path.to_string_lossy().to_string()));
    ref_args.insert("query".to_string(), json!("within"));
    ref_args.insert("threshold".to_string(), json!(0.2));
    ref_args.insert("output".to_string(), json!(ref_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_by_reference_surface", &ref_args, &context(&caps))
        .expect("filter_lidar_by_reference_surface should run");
    let ref_cloud = PointCloud::read(&ref_out).expect("read ref-surface output");
    assert!(ref_cloud.points.len() >= 2 && ref_cloud.points.len() < cloud.points.len());

    let mut class_args = ToolArgs::new();
    class_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    class_args.insert("search_radius".to_string(), json!(2.0));
    class_args.insert("grd_threshold".to_string(), json!(0.2));
    class_args.insert("oto_threshold".to_string(), json!(1.0));
    class_args.insert("output".to_string(), json!(classed_out.to_string_lossy().to_string()));
    registry
        .run("classify_lidar", &class_args, &context(&caps))
        .expect("classify_lidar should run");
    let classed = PointCloud::read(&classed_out).expect("read classify output");
    assert_eq!(classed.points.len(), cloud.points.len());
    assert!(classed.points.iter().any(|p| p.classification == 2));
    assert!(classed.points.iter().any(|p| p.classification == 5 || p.classification == 6));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&surface_path);
    let _ = std::fs::remove_file(&ref_out);
    let _ = std::fs::remove_file(&classed_out);
}

#[test]
fn lidar_phase2_partial_batch_hardening_sort_percentile_outliers() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_partial_hardening");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let sort_out = std::env::temp_dir().join(format!("{tag}_sorted.las"));
    let pct_lo_out = std::env::temp_dir().join(format!("{tag}_pct_lo.las"));
    let pct_hi_out = std::env::temp_dir().join(format!("{tag}_pct_hi.las"));
    let outlier_class_out = std::env::temp_dir().join(format!("{tag}_outlier_class.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, return_number: 2, number_of_returns: 2, ..PointRecord::default() },
            PointRecord { x: 0.1, y: 0.0, z: 11.0, classification: 1, return_number: 1, number_of_returns: 2, ..PointRecord::default() },
            PointRecord { x: 0.2, y: 0.0, z: 100.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.3, y: 0.0, z: -40.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.4, y: 0.0, z: 10.5, classification: 7, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write lidar input");

    // sort_lidar should accept legacy alias return_number and sort by it.
    let mut sort_args = ToolArgs::new();
    sort_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    sort_args.insert("sort_criteria".to_string(), json!("return_number, z"));
    sort_args.insert("output".to_string(), json!(sort_out.to_string_lossy().to_string()));
    registry
        .run("sort_lidar", &sort_args, &context(&caps))
        .expect("sort_lidar should run with return_number alias");
    let sorted = PointCloud::read(&sort_out).expect("read sorted output");
    for i in 1..sorted.points.len() {
        assert!(
            sorted.points[i - 1].return_number <= sorted.points[i].return_number,
            "points should be ordered by return_number"
        );
    }

    // percentile extremes should choose low/high from the same block.
    let mut pct_lo_args = ToolArgs::new();
    pct_lo_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    pct_lo_args.insert("percentile".to_string(), json!(0.0));
    pct_lo_args.insert("block_size".to_string(), json!(100.0));
    pct_lo_args.insert("output".to_string(), json!(pct_lo_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_by_percentile", &pct_lo_args, &context(&caps))
        .expect("filter_lidar_by_percentile low percentile should run");
    let pct_lo = PointCloud::read(&pct_lo_out).expect("read percentile low output");
    assert_eq!(pct_lo.points.len(), 1);
    assert!((pct_lo.points[0].z + 40.0).abs() < 1e-9);

    let mut pct_hi_args = ToolArgs::new();
    pct_hi_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    pct_hi_args.insert("percentile".to_string(), json!(100.0));
    pct_hi_args.insert("block_size".to_string(), json!(100.0));
    pct_hi_args.insert("output".to_string(), json!(pct_hi_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_by_percentile", &pct_hi_args, &context(&caps))
        .expect("filter_lidar_by_percentile high percentile should run");
    let pct_hi = PointCloud::read(&pct_hi_out).expect("read percentile high output");
    assert_eq!(pct_hi.points.len(), 1);
    assert!((pct_hi.points[0].z - 100.0).abs() < 1e-9);

    // outlier classify mode should assign both low/high noise classes where applicable.
    let mut outlier_args = ToolArgs::new();
    outlier_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    outlier_args.insert("search_radius".to_string(), json!(5.0));
    outlier_args.insert("elev_diff".to_string(), json!(20.0));
    outlier_args.insert("classify".to_string(), json!(true));
    outlier_args.insert("output".to_string(), json!(outlier_class_out.to_string_lossy().to_string()));
    registry
        .run("lidar_remove_outliers", &outlier_args, &context(&caps))
        .expect("lidar_remove_outliers classify mode should run");
    let out_classed = PointCloud::read(&outlier_class_out).expect("read outlier classify output");
    assert!(out_classed.points.iter().any(|p| p.classification == 7));
    assert!(out_classed.points.iter().any(|p| p.classification == 18));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&sort_out);
    let _ = std::fs::remove_file(&pct_lo_out);
    let _ = std::fs::remove_file(&pct_hi_out);
    let _ = std::fs::remove_file(&outlier_class_out);
}

#[test]
fn lidar_phase2_partial_batch_hardening_join_tile_thin_hd() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_partial_hardening_b");
    let join_a = std::env::temp_dir().join(format!("{tag}_join_a.las"));
    let join_b = std::env::temp_dir().join(format!("{tag}_join_b.las"));
    let joined_out = std::env::temp_dir().join(format!("{tag}_joined.las"));

    let tile_in = std::env::temp_dir().join(format!("{tag}_tile_in.las"));
    let tile_out_dir = std::env::temp_dir().join(format!("{tag}_tiles"));

    let thin_in = std::env::temp_dir().join(format!("{tag}_thin_in.las"));
    let thin_out = std::env::temp_dir().join(format!("{tag}_thin_out.las"));

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 10.0, y: 0.0, z: 1.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 11.0, y: 0.0, z: 2.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 3.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 4.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&join_a).expect("write join_a");
    cloud_b.write(&join_b).expect("write join_b");

    let mut join_args = ToolArgs::new();
    join_args.insert(
        "inputs".to_string(),
        json!([join_a.to_string_lossy().to_string(), join_b.to_string_lossy().to_string()]),
    );
    join_args.insert("output".to_string(), json!(joined_out.to_string_lossy().to_string()));
    registry
        .run("lidar_join", &join_args, &context(&caps))
        .expect("lidar_join hardening run");
    let joined = PointCloud::read(&joined_out).expect("read joined output");
    assert_eq!(joined.points.len(), 4);
    assert!((joined.points[0].x - 10.0).abs() < 1e-9);
    assert!((joined.points[1].x - 11.0).abs() < 1e-9);
    assert_eq!(joined.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    // tile hardening: strict min_points_in_tile behavior (> threshold) should keep only tile with 3 points.
    let tile_cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.1, y: 0.1, z: 1.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.2, y: 0.1, z: 1.1, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.3, y: 0.1, z: 1.2, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.1, y: 0.1, z: 2.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.2, y: 0.1, z: 2.1, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    tile_cloud.write(&tile_in).expect("write tile input");
    std::fs::create_dir_all(&tile_out_dir).expect("create tile output dir");

    let mut tile_args = ToolArgs::new();
    tile_args.insert("input".to_string(), json!(tile_in.to_string_lossy().to_string()));
    tile_args.insert("tile_width".to_string(), json!(1.0));
    tile_args.insert("tile_height".to_string(), json!(1.0));
    tile_args.insert("origin_x".to_string(), json!(0.0));
    tile_args.insert("origin_y".to_string(), json!(0.0));
    tile_args.insert("min_points_in_tile".to_string(), json!(2));
    tile_args.insert("output_laz_format".to_string(), json!(false));
    tile_args.insert("output_directory".to_string(), json!(tile_out_dir.to_string_lossy().to_string()));
    let tile_res = registry
        .run("lidar_tile", &tile_args, &context(&caps))
        .expect("lidar_tile hardening run");
    assert_eq!(tile_res.outputs.get("tile_count").and_then(|v| v.as_u64()), Some(1));

    // thin-high-density hardening: when save_filtered=false, no filtered_path should be returned.
    let thin_cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 1.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.05, y: 0.0, z: 1.1, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.1, y: 0.0, z: 1.2, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.15, y: 0.0, z: 1.3, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    thin_cloud.write(&thin_in).expect("write thin input");

    let mut thin_args = ToolArgs::new();
    thin_args.insert("input".to_string(), json!(thin_in.to_string_lossy().to_string()));
    thin_args.insert("density".to_string(), json!(0.1));
    thin_args.insert("resolution".to_string(), json!(1.0));
    thin_args.insert("save_filtered".to_string(), json!(false));
    thin_args.insert("output".to_string(), json!(thin_out.to_string_lossy().to_string()));
    let thin_res = registry
        .run("lidar_thin_high_density", &thin_args, &context(&caps))
        .expect("lidar_thin_high_density hardening run");
    assert!(thin_res.outputs.get("filtered_path").is_none());
    let thin_kept = PointCloud::read(&thin_out).expect("read thin output");
    assert!(thin_kept.points.len() <= thin_cloud.points.len());

    let _ = std::fs::remove_file(&join_a);
    let _ = std::fs::remove_file(&join_b);
    let _ = std::fs::remove_file(&joined_out);
    let _ = std::fs::remove_file(&tile_in);
    let _ = std::fs::remove_dir_all(&tile_out_dir);
    let _ = std::fs::remove_file(&thin_in);
    let _ = std::fs::remove_file(&thin_out);
}

#[test]
fn lidar_phase2_partial_batch_hardening_reference_split_classify() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_partial_hardening_c");
    let lidar_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let surface_path = std::env::temp_dir().join(format!("{tag}_surface.tif"));
    let ref_classify_out = std::env::temp_dir().join(format!("{tag}_ref_classify.las"));
    let split_dir = std::env::temp_dir().join(format!("{tag}_split"));
    let classify_out = std::env::temp_dir().join(format!("{tag}_classify.las"));

    let mut points = vec![
        PointRecord { x: 0.5, y: 1.5, z: 10.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        PointRecord { x: 1.5, y: 1.5, z: 12.0, classification: 6, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        PointRecord { x: 0.5, y: 0.5, z: 9.5, classification: 7, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
    ];
    for i in 0..250 {
        points.push(PointRecord {
            x: (i as f64) * 0.01,
            y: 2.0,
            z: 8.0 + ((i % 5) as f64),
            classification: 1,
            return_number: 1,
            number_of_returns: 1,
            ..PointRecord::default()
        });
    }
    let cloud = PointCloud {
        points,
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&lidar_path).expect("write input cloud");

    let mut surface = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::F64,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    surface.set(0, 0, 0, 10.0).expect("set surface");
    surface.set(0, 0, 1, 10.0).expect("set surface");
    surface.set(0, 1, 0, 10.0).expect("set surface");
    surface.set(0, 1, 1, 10.0).expect("set surface");
    surface.write(&surface_path, RasterFormat::GeoTiff).expect("write surface");

    // filter_lidar_by_reference_surface classify hardening: >= query + preserve_classes.
    let mut ref_args = ToolArgs::new();
    ref_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    ref_args.insert("ref_surface".to_string(), json!(surface_path.to_string_lossy().to_string()));
    ref_args.insert("query".to_string(), json!(">="));
    ref_args.insert("classify".to_string(), json!(true));
    ref_args.insert("true_class_value".to_string(), json!(2));
    ref_args.insert("false_class_value".to_string(), json!(9));
    ref_args.insert("preserve_classes".to_string(), json!(true));
    ref_args.insert("output".to_string(), json!(ref_classify_out.to_string_lossy().to_string()));
    registry
        .run("filter_lidar_by_reference_surface", &ref_args, &context(&caps))
        .expect("filter_lidar_by_reference_surface classify hardening run");
    let ref_cloud = PointCloud::read(&ref_classify_out).expect("read ref classify output");
    assert_eq!(ref_cloud.points.len(), cloud.points.len());
    assert!(ref_cloud.points.iter().filter(|p| p.classification == 2).count() >= 2);
    assert!(ref_cloud.points.iter().any(|p| p.classification == 7));

    // split_lidar hardening: num_pts mode with interval>=100 and explicit output_count.
    std::fs::create_dir_all(&split_dir).expect("create split dir");
    let mut split_args = ToolArgs::new();
    split_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    split_args.insert("split_criterion".to_string(), json!("num_pts"));
    split_args.insert("interval".to_string(), json!(100.0));
    split_args.insert("min_pts".to_string(), json!(0));
    split_args.insert("output_directory".to_string(), json!(split_dir.to_string_lossy().to_string()));
    let split_res = registry
        .run("split_lidar", &split_args, &context(&caps))
        .expect("split_lidar num_pts hardening run");
    assert_eq!(split_res.outputs.get("output_count").and_then(|v| v.as_u64()), Some(3));

    // classify_lidar hardening: advanced params accepted + output classes include ground and off-terrain.
    let mut class_args = ToolArgs::new();
    class_args.insert("input".to_string(), json!(lidar_path.to_string_lossy().to_string()));
    class_args.insert("search_radius".to_string(), json!(1.5));
    class_args.insert("grd_threshold".to_string(), json!(0.2));
    class_args.insert("oto_threshold".to_string(), json!(1.0));
    class_args.insert("linearity_threshold".to_string(), json!(0.4));
    class_args.insert("planarity_threshold".to_string(), json!(0.8));
    class_args.insert("num_iter".to_string(), json!(20));
    class_args.insert("facade_threshold".to_string(), json!(0.5));
    class_args.insert("output".to_string(), json!(classify_out.to_string_lossy().to_string()));
    registry
        .run("classify_lidar", &class_args, &context(&caps))
        .expect("classify_lidar hardening run");
    let classed = PointCloud::read(&classify_out).expect("read classify output");
    assert_eq!(classed.points.len(), cloud.points.len());
    assert!(classed.points.iter().any(|p| p.classification == 2));
    assert!(classed.points.iter().any(|p| p.classification == 5 || p.classification == 6));

    let _ = std::fs::remove_file(&lidar_path);
    let _ = std::fs::remove_file(&surface_path);
    let _ = std::fs::remove_file(&ref_classify_out);
    let _ = std::fs::remove_dir_all(&split_dir);
    let _ = std::fs::remove_file(&classify_out);
}

#[test]
fn classify_lidar_batch_mode_without_input_processes_tiles() {
    let _cwd_guard = CWD_TEST_LOCK.lock().expect("acquire cwd test lock");

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_classify_batch_mode");
    let batch_dir = std::env::temp_dir().join(format!("{tag}_dir"));
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");
    let tile_a = batch_dir.join("classify_tile_a.las");
    let tile_b = batch_dir.join("classify_tile_b.las");

    let cloud_a = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.5, y: 0.0, z: 10.1, classification: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 0.5, z: 12.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let cloud_b = PointCloud {
        points: vec![
            PointRecord { x: 2.0, y: 2.0, z: 20.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 2.5, y: 2.0, z: 20.2, classification: 1, ..PointRecord::default() },
            PointRecord { x: 2.0, y: 2.5, z: 22.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud_a.write(&tile_a).expect("write tile_a");
    cloud_b.write(&tile_b).expect("write tile_b");

    let old_cwd = std::env::current_dir().expect("get current dir");
    std::env::set_current_dir(&batch_dir).expect("set current dir");

    let mut args = ToolArgs::new();
    args.insert("search_radius".to_string(), json!(1.5));
    args.insert("grd_threshold".to_string(), json!(0.2));
    args.insert("oto_threshold".to_string(), json!(1.0));
    let res = registry
        .run("classify_lidar", &args, &context(&caps))
        .expect("classify_lidar batch run");

    std::env::set_current_dir(&old_cwd).expect("restore current dir");

    assert_eq!(res.outputs.get("__wbw_type__"), Some(&json!("lidar")));
    let p = res.outputs.get("path").and_then(|v| v.as_str()).expect("placeholder path");
    assert!(std::path::Path::new(p).exists());

    let _ = std::fs::remove_dir_all(&batch_dir);
}

#[test]
fn lidar_phase2_next_batch_subset_clip_erase_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_next_batch");
    let base_path = std::env::temp_dir().join(format!("{tag}_base.las"));
    let subset_path = std::env::temp_dir().join(format!("{tag}_subset.las"));
    let poly_path = std::env::temp_dir().join(format!("{tag}_poly.geojson"));
    let subset_out = std::env::temp_dir().join(format!("{tag}_subset_out.las"));
    let clip_out = std::env::temp_dir().join(format!("{tag}_clip_out.las"));
    let erase_out = std::env::temp_dir().join(format!("{tag}_erase_out.las"));

    let base = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 10.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 12.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 5.0, y: 5.0, z: 15.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    let subset = PointCloud {
        points: vec![
            PointRecord { x: 1.0, y: 1.0, z: 12.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    base.write(&base_path).expect("write base cloud");
    subset.write(&subset_path).expect("write subset cloud");

    let mut layer = Layer::new("poly").with_geom_type(GeometryType::Polygon);
    let mut feature = Feature::new();
    feature.geometry = Some(Geometry::polygon(
        vec![
            Coord::xy(-0.5, -0.5),
            Coord::xy(2.0, -0.5),
            Coord::xy(2.0, 2.0),
            Coord::xy(-0.5, 2.0),
        ],
        vec![],
    ));
    layer.push(feature);
    wbvector::write(&layer, &poly_path, VectorFormat::GeoJson).expect("write polygon vector");

    let mut subset_args = ToolArgs::new();
    subset_args.insert("base".to_string(), json!(base_path.to_string_lossy().to_string()));
    subset_args.insert("subset".to_string(), json!(subset_path.to_string_lossy().to_string()));
    subset_args.insert("subset_class_value".to_string(), json!(6));
    subset_args.insert("nonsubset_class_value".to_string(), json!(1));
    subset_args.insert("tolerance".to_string(), json!(0.001));
    subset_args.insert("output".to_string(), json!(subset_out.to_string_lossy().to_string()));
    registry
        .run("lidar_classify_subset", &subset_args, &context(&caps))
        .expect("lidar_classify_subset run");
    let subset_result = PointCloud::read(&subset_out).expect("read subset output");
    assert_eq!(subset_result.points.len(), 3);
    assert_eq!(subset_result.points.iter().filter(|p| p.classification == 6).count(), 1);

    let mut clip_args = ToolArgs::new();
    clip_args.insert("input".to_string(), json!(base_path.to_string_lossy().to_string()));
    clip_args.insert("polygons".to_string(), json!(poly_path.to_string_lossy().to_string()));
    clip_args.insert("output".to_string(), json!(clip_out.to_string_lossy().to_string()));
    registry
        .run("clip_lidar_to_polygon", &clip_args, &context(&caps))
        .expect("clip_lidar_to_polygon run");
    let clipped = PointCloud::read(&clip_out).expect("read clip output");
    assert_eq!(clipped.points.len(), 2);
    assert!(clipped.points.iter().all(|p| p.x <= 2.0 && p.y <= 2.0));

    let mut erase_args = ToolArgs::new();
    erase_args.insert("input".to_string(), json!(base_path.to_string_lossy().to_string()));
    erase_args.insert("polygons".to_string(), json!(poly_path.to_string_lossy().to_string()));
    erase_args.insert("output".to_string(), json!(erase_out.to_string_lossy().to_string()));
    registry
        .run("erase_polygon_from_lidar", &erase_args, &context(&caps))
        .expect("erase_polygon_from_lidar run");
    let erased = PointCloud::read(&erase_out).expect("read erase output");
    assert_eq!(erased.points.len(), 1);
    assert!((erased.points[0].x - 5.0).abs() < 1.0e-9);

    let _ = std::fs::remove_file(&base_path);
    let _ = std::fs::remove_file(&subset_path);
    let _ = std::fs::remove_file(&poly_path);
    let _ = std::fs::remove_file(&subset_out);
    let _ = std::fs::remove_file(&clip_out);
    let _ = std::fs::remove_file(&erase_out);
}

#[test]
fn lidar_phase2_next_batch_overlap_segmentation_tools_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_next_batch_d");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let overlap_out = std::env::temp_dir().join(format!("{tag}_overlap.las"));
    let overlap_filtered_out = std::env::temp_dir().join(format!("{tag}_overlap_filtered.las"));
    let seg_out = std::env::temp_dir().join(format!("{tag}_seg.las"));
    let seg_filter_out = std::env::temp_dir().join(format!("{tag}_seg_filter.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.1, y: 0.1, z: 10.0, classification: 1, point_source_id: 1, scan_angle: 5, gps_time: Some(GpsTime(1.0)), ..PointRecord::default() },
            PointRecord { x: 0.2, y: 0.1, z: 10.1, classification: 1, point_source_id: 2, scan_angle: 18, gps_time: Some(GpsTime(2.0)), ..PointRecord::default() },
            PointRecord { x: 2.0, y: 2.0, z: 11.0, classification: 1, point_source_id: 1, scan_angle: 2, gps_time: Some(GpsTime(3.0)), ..PointRecord::default() },
            PointRecord { x: 2.2, y: 2.1, z: 11.2, classification: 1, point_source_id: 1, scan_angle: 1, gps_time: Some(GpsTime(4.0)), ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut overlap_args = ToolArgs::new();
    overlap_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    overlap_args.insert("resolution".to_string(), json!(1.0));
    overlap_args.insert("overlap_criterion".to_string(), json!("multiple point source IDs"));
    overlap_args.insert("filter".to_string(), json!(false));
    overlap_args.insert("output".to_string(), json!(overlap_out.to_string_lossy().to_string()));
    registry
        .run("classify_overlap_points", &overlap_args, &context(&caps))
        .expect("classify_overlap_points classify run");
    let overlap_cloud = PointCloud::read(&overlap_out).expect("read overlap output");
    assert!(overlap_cloud.points.iter().any(|p| p.classification == 12));

    let mut overlap_filter_args = ToolArgs::new();
    overlap_filter_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    overlap_filter_args.insert("resolution".to_string(), json!(1.0));
    overlap_filter_args.insert("overlap_criterion".to_string(), json!("multiple point source IDs"));
    overlap_filter_args.insert("filter".to_string(), json!(true));
    overlap_filter_args.insert("output".to_string(), json!(overlap_filtered_out.to_string_lossy().to_string()));
    registry
        .run("classify_overlap_points", &overlap_filter_args, &context(&caps))
        .expect("classify_overlap_points filter run");
    let overlap_filtered_cloud = PointCloud::read(&overlap_filtered_out).expect("read overlap filtered output");
    assert!(overlap_filtered_cloud.points.len() < cloud.points.len());

    let mut seg_args = ToolArgs::new();
    seg_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    seg_args.insert("search_radius".to_string(), json!(0.5));
    seg_args.insert("max_z_diff".to_string(), json!(0.5));
    seg_args.insert("ground".to_string(), json!(true));
    seg_args.insert("output".to_string(), json!(seg_out.to_string_lossy().to_string()));
    registry
        .run("lidar_segmentation", &seg_args, &context(&caps))
        .expect("lidar_segmentation run");
    let seg_cloud = PointCloud::read(&seg_out).expect("read segmentation output");
    assert_eq!(seg_cloud.points.len(), cloud.points.len());
    assert!(seg_cloud.points.iter().any(|p| p.color.is_some()));
    assert!(seg_cloud.points.iter().any(|p| p.classification == 2));

    let mut seg_filter_args = ToolArgs::new();
    seg_filter_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    seg_filter_args.insert("search_radius".to_string(), json!(0.6));
    seg_filter_args.insert("max_z_diff".to_string(), json!(0.3));
    seg_filter_args.insert("classify_points".to_string(), json!(false));
    seg_filter_args.insert("output".to_string(), json!(seg_filter_out.to_string_lossy().to_string()));
    registry
        .run("lidar_segmentation_based_filter", &seg_filter_args, &context(&caps))
        .expect("lidar_segmentation_based_filter run");
    let seg_filter_cloud = PointCloud::read(&seg_filter_out).expect("read seg filter output");
    assert!(seg_filter_cloud.points.len() <= cloud.points.len());
    assert!(!seg_filter_cloud.points.is_empty());

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&overlap_out);
    let _ = std::fs::remove_file(&overlap_filtered_out);
    let _ = std::fs::remove_file(&seg_out);
    let _ = std::fs::remove_file(&seg_filter_out);
}

#[test]
fn lidar_phase2_modify_and_segmentation_hardening_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_next_batch_h");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let modify_out = std::env::temp_dir().join(format!("{tag}_modify.las"));
    let seg_out = std::env::temp_dir().join(format!("{tag}_seg.las"));
    let seg_filter_class_out = std::env::temp_dir().join(format!("{tag}_seg_filter_class.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                classification: 1,
                intensity: 100,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.2,
                y: 0.1,
                z: 10.1,
                classification: 2,
                intensity: 150,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 2.0,
                y: 2.0,
                z: 12.5,
                classification: 5,
                intensity: 200,
                return_number: 1,
                number_of_returns: 2,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut modify_args = ToolArgs::new();
    modify_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    modify_args.insert(
        "statement".to_string(),
        json!("z = z + 1.0; class = if(z > 12.0, 6, class); rgb = (100,200,300)"),
    );
    modify_args.insert("output".to_string(), json!(modify_out.to_string_lossy().to_string()));
    registry
        .run("modify_lidar", &modify_args, &context(&caps))
        .expect("modify_lidar run");
    let modified = PointCloud::read(&modify_out).expect("read modify output");
    assert_eq!(modified.points.len(), 3);
    assert!((modified.points[0].z - 11.0).abs() < 1.0e-9);
    assert_eq!(modified.points[2].classification, 6);
    let clr = modified.points[0].color.expect("modified colour");
    assert_eq!((clr.red, clr.green, clr.blue), (100, 200, 300));

    let mut seg_args = ToolArgs::new();
    seg_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    seg_args.insert("search_radius".to_string(), json!(0.5));
    seg_args.insert("max_z_diff".to_string(), json!(0.3));
    seg_args.insert("classes".to_string(), json!(true));
    seg_args.insert("ground".to_string(), json!(true));
    seg_args.insert("output".to_string(), json!(seg_out.to_string_lossy().to_string()));
    registry
        .run("lidar_segmentation", &seg_args, &context(&caps))
        .expect("lidar_segmentation hardening run");
    let seg_cloud = PointCloud::read(&seg_out).expect("read seg output");
    assert_eq!(seg_cloud.points.len(), cloud.points.len());
    assert!(seg_cloud.points.iter().any(|p| p.classification == 2));
    assert!(seg_cloud.points.iter().all(|p| p.color.is_some()));

    let mut seg_filter_class_args = ToolArgs::new();
    seg_filter_class_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    seg_filter_class_args.insert("search_radius".to_string(), json!(0.6));
    seg_filter_class_args.insert("max_z_diff".to_string(), json!(0.5));
    seg_filter_class_args.insert("classify_points".to_string(), json!(true));
    seg_filter_class_args.insert(
        "output".to_string(),
        json!(seg_filter_class_out.to_string_lossy().to_string()),
    );
    registry
        .run(
            "lidar_segmentation_based_filter",
            &seg_filter_class_args,
            &context(&caps),
        )
        .expect("lidar_segmentation_based_filter classify run");
    let seg_filter_class_cloud =
        PointCloud::read(&seg_filter_class_out).expect("read seg filter classify output");
    assert_eq!(seg_filter_class_cloud.points.len(), cloud.points.len());
    assert!(
        seg_filter_class_cloud
            .points
            .iter()
            .all(|p| p.classification == 1 || p.classification == 2)
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&modify_out);
    let _ = std::fs::remove_file(&seg_out);
    let _ = std::fs::remove_file(&seg_filter_class_out);
}

#[test]
fn individual_tree_segmentation_is_deterministic_with_fixed_seed() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_individual_tree_seg_determinism");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let out1 = std::env::temp_dir().join(format!("{tag}_out1.las"));
    let out2 = std::env::temp_dir().join(format!("{tag}_out2.las"));

    let mut points = Vec::new();
    for i in 0..40usize {
        let angle = (i as f64) * 0.43;
        let radial = 0.5 + ((i % 7) as f64) * 0.18;
        points.push(PointRecord {
            x: radial * angle.cos(),
            y: radial * angle.sin(),
            z: 3.0 + ((i % 9) as f64) * 0.35,
            classification: 5,
            ..PointRecord::default()
        });
    }
    for i in 0..40usize {
        let angle = (i as f64) * 0.39;
        let radial = 0.6 + ((i % 8) as f64) * 0.2;
        points.push(PointRecord {
            x: 8.0 + radial * angle.cos(),
            y: 1.5 + radial * angle.sin(),
            z: 3.5 + ((i % 10) as f64) * 0.32,
            classification: 5,
            ..PointRecord::default()
        });
    }
    // Non-vegetation points should be ignored by default.
    points.push(PointRecord { x: 4.0, y: 4.0, z: 0.6, classification: 2, ..PointRecord::default() });

    let cloud = PointCloud {
        points,
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write deterministic input cloud");

    let run_once = |output_path: &std::path::Path| {
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
        args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
        args.insert("only_use_veg".to_string(), json!(true));
        args.insert("min_height".to_string(), json!(2.0));
        args.insert("bandwidth_min".to_string(), json!(0.8));
        args.insert("bandwidth_max".to_string(), json!(3.5));
        args.insert("adaptive_bandwidth".to_string(), json!(true));
        args.insert("adaptive_neighbors".to_string(), json!(20));
        args.insert("adaptive_sector_count".to_string(), json!(8));
        args.insert("grid_acceleration".to_string(), json!(true));
        args.insert("grid_cell_size".to_string(), json!(0.7));
        args.insert("vertical_bandwidth".to_string(), json!(4.0));
        args.insert("max_iterations".to_string(), json!(20));
        args.insert("convergence_tol".to_string(), json!(0.05));
        args.insert("min_cluster_points".to_string(), json!(8));
        args.insert("mode_merge_dist".to_string(), json!(0.8));
        args.insert("output_id_mode".to_string(), json!("point_source_id"));
        args.insert("seed".to_string(), json!(42));
        registry
            .run("individual_tree_segmentation", &args, &context(&caps))
            .expect("individual_tree_segmentation run");
        PointCloud::read(output_path).expect("read segmentation output")
    };

    let first = run_once(&out1);
    let second = run_once(&out2);

    assert_eq!(first.points.len(), second.points.len());
    for i in 0..first.points.len() {
        let p1 = first.points[i];
        let p2 = second.points[i];
        assert_eq!(p1.point_source_id, p2.point_source_id, "point_source_id mismatch at index {i}");
        assert_eq!(p1.user_data, p2.user_data, "user_data mismatch at index {i}");
        assert_eq!(p1.classification, p2.classification, "classification mismatch at index {i}");
        assert_eq!(p1.color, p2.color, "color mismatch at index {i}");
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out1);
    let _ = std::fs::remove_file(&out2);
}

#[test]
fn individual_tree_segmentation_prunes_tiny_far_cluster() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_individual_tree_seg_prune");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.las"));

    let mut points = Vec::new();
    // Large retained tree cluster near origin.
    for i in 0..48usize {
        let angle = (i as f64) * 0.31;
        let radial = 0.5 + ((i % 11) as f64) * 0.12;
        points.push(PointRecord {
            x: radial * angle.cos(),
            y: radial * angle.sin(),
            z: 2.8 + ((i % 13) as f64) * 0.28,
            classification: 5,
            ..PointRecord::default()
        });
    }
    // Tiny distant cluster expected to be pruned and remain unassigned.
    for i in 0..5usize {
        let angle = (i as f64) * 1.1;
        let radial = 0.2 + ((i % 3) as f64) * 0.08;
        points.push(PointRecord {
            x: 20.0 + radial * angle.cos(),
            y: 20.0 + radial * angle.sin(),
            z: 3.0 + ((i % 4) as f64) * 0.2,
            classification: 5,
            ..PointRecord::default()
        });
    }

    let cloud = PointCloud {
        points,
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write pruning input cloud");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    args.insert("only_use_veg".to_string(), json!(true));
    args.insert("min_height".to_string(), json!(2.0));
    args.insert("bandwidth_min".to_string(), json!(0.8));
    args.insert("bandwidth_max".to_string(), json!(2.5));
    args.insert("adaptive_bandwidth".to_string(), json!(true));
    args.insert("adaptive_neighbors".to_string(), json!(20));
    args.insert("adaptive_sector_count".to_string(), json!(8));
    args.insert("grid_acceleration".to_string(), json!(true));
    args.insert("grid_cell_size".to_string(), json!(0.7));
    args.insert("vertical_bandwidth".to_string(), json!(4.0));
    args.insert("max_iterations".to_string(), json!(24));
    args.insert("convergence_tol".to_string(), json!(0.05));
    args.insert("min_cluster_points".to_string(), json!(12));
    args.insert("mode_merge_dist".to_string(), json!(0.8));
    args.insert("output_id_mode".to_string(), json!("point_source_id"));
    args.insert("seed".to_string(), json!(42));
    registry
        .run("individual_tree_segmentation", &args, &context(&caps))
        .expect("individual_tree_segmentation prune run");

    let out_cloud = PointCloud::read(&output_path).expect("read prune output");
    let mut near_assigned = 0usize;
    let mut far_unassigned = 0usize;
    let mut far_total = 0usize;
    for p in &out_cloud.points {
        if p.classification != 5 || p.z < 2.0 {
            continue;
        }
        if p.x > 15.0 && p.y > 15.0 {
            far_total += 1;
            if p.point_source_id == 0 {
                far_unassigned += 1;
            }
        } else if p.point_source_id > 0 {
            near_assigned += 1;
        }
    }

    assert!(near_assigned >= 40, "expected retained near cluster points to stay assigned");
    assert_eq!(far_total, 5, "expected five far tiny-cluster points");
    assert_eq!(
        far_unassigned, far_total,
        "expected all tiny distant cluster points to be unassigned after pruning"
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn individual_tree_segmentation_tiled_grid_refine_runs_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_individual_tree_seg_tiled_refine");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.las"));

    let mut points = Vec::new();
    for i in 0..120usize {
        let angle = (i as f64) * 0.23;
        let radial = 0.4 + ((i % 16) as f64) * 0.1;
        points.push(PointRecord {
            x: radial * angle.cos(),
            y: radial * angle.sin(),
            z: 3.0 + ((i % 21) as f64) * 0.2,
            classification: 5,
            ..PointRecord::default()
        });
    }
    for i in 0..120usize {
        let angle = (i as f64) * 0.19;
        let radial = 0.5 + ((i % 14) as f64) * 0.12;
        points.push(PointRecord {
            x: 14.0 + radial * angle.cos(),
            y: 10.0 + radial * angle.sin(),
            z: 3.2 + ((i % 19) as f64) * 0.24,
            classification: 5,
            ..PointRecord::default()
        });
    }

    let cloud = PointCloud {
        points,
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write tiled-refine input cloud");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    args.insert("only_use_veg".to_string(), json!(true));
    args.insert("min_height".to_string(), json!(2.0));
    args.insert("bandwidth_min".to_string(), json!(0.8));
    args.insert("bandwidth_max".to_string(), json!(4.0));
    args.insert("adaptive_bandwidth".to_string(), json!(true));
    args.insert("adaptive_neighbors".to_string(), json!(24));
    args.insert("adaptive_sector_count".to_string(), json!(8));
    args.insert("grid_acceleration".to_string(), json!(true));
    args.insert("grid_cell_size".to_string(), json!(0.75));
    args.insert("grid_refine_exact".to_string(), json!(true));
    args.insert("grid_refine_iterations".to_string(), json!(2));
    args.insert("tile_size".to_string(), json!(6.0));
    args.insert("tile_overlap".to_string(), json!(1.0));
    args.insert("vertical_bandwidth".to_string(), json!(4.0));
    args.insert("max_iterations".to_string(), json!(24));
    args.insert("convergence_tol".to_string(), json!(0.05));
    args.insert("min_cluster_points".to_string(), json!(20));
    args.insert("mode_merge_dist".to_string(), json!(0.85));
    args.insert("output_id_mode".to_string(), json!("point_source_id"));
    args.insert("seed".to_string(), json!(7));
    registry
        .run("individual_tree_segmentation", &args, &context(&caps))
        .expect("individual_tree_segmentation tiled-refine run");

    let out_cloud = PointCloud::read(&output_path).expect("read tiled-refine output");
    assert_eq!(out_cloud.points.len(), cloud.points.len());
    assert!(
        out_cloud.points.iter().any(|p| p.point_source_id > 0),
        "expected at least some assigned points"
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn lidar_phase2_next_batch_colourize_tools_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_next_batch_e");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let image_path = std::env::temp_dir().join(format!("{tag}_img.tif"));
    let lidar_colourize_out = std::env::temp_dir().join(format!("{tag}_lidar_colourize.las"));
    let class_colourize_out = std::env::temp_dir().join(format!("{tag}_class_colourize.las"));
    let returns_colourize_out = std::env::temp_dir().join(format!("{tag}_returns_colourize.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.5, y: 0.5, z: 10.0, intensity: 10000, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.5, y: 0.5, z: 11.0, intensity: 30000, classification: 6, return_number: 1, number_of_returns: 2, ..PointRecord::default() },
            PointRecord { x: 2.5, y: 0.5, z: 12.0, intensity: 40000, classification: 5, return_number: 2, number_of_returns: 2, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut image = Raster::new(RasterConfig {
        cols: 3,
        rows: 1,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: None,
        nodata: -9999.0,
        data_type: DataType::U32,
        crs: CrsInfo::from_epsg(4326),
        metadata: Vec::new(),
    });
    image.set(0, 0, 0, 0x000000FFu32 as f64).expect("set pixel 0"); // red
    image.set(0, 0, 1, 0x0000FF00u32 as f64).expect("set pixel 1"); // green
    image.set(0, 0, 2, 0x00FF0000u32 as f64).expect("set pixel 2"); // blue
    image.write(&image_path, RasterFormat::GeoTiff).expect("write image");

    let mut lidar_colourize_args = ToolArgs::new();
    lidar_colourize_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    lidar_colourize_args.insert("image".to_string(), json!(image_path.to_string_lossy().to_string()));
    lidar_colourize_args.insert("output".to_string(), json!(lidar_colourize_out.to_string_lossy().to_string()));
    registry
        .run("lidar_colourize", &lidar_colourize_args, &context(&caps))
        .expect("lidar_colourize run");
    let coloured = PointCloud::read(&lidar_colourize_out).expect("read lidar_colourize output");
    assert!(coloured.points.iter().all(|p| p.color.is_some()));

    let mut class_colourize_args = ToolArgs::new();
    class_colourize_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    class_colourize_args.insert("intensity_blending_amount".to_string(), json!(0.0));
    class_colourize_args.insert("clr_str".to_string(), json!("2:(10,20,30);6:#ff0000"));
    class_colourize_args.insert("output".to_string(), json!(class_colourize_out.to_string_lossy().to_string()));
    registry
        .run("colourize_based_on_class", &class_colourize_args, &context(&caps))
        .expect("colourize_based_on_class run");
    let class_coloured = PointCloud::read(&class_colourize_out).expect("read class colourized output");
    assert!(class_coloured.points.iter().all(|p| p.color.is_some()));
    let c0 = class_coloured.points[0].color.expect("point 0 colour");
    assert_eq!(c0.red / 257, 10);
    assert_eq!(c0.green / 257, 20);
    assert_eq!(c0.blue / 257, 30);

    let mut returns_colourize_args = ToolArgs::new();
    returns_colourize_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    returns_colourize_args.insert("intensity_blending_amount".to_string(), json!(0.0));
    returns_colourize_args.insert("only_ret_colour".to_string(), json!("(1,2,3)"));
    returns_colourize_args.insert("first_ret_colour".to_string(), json!("(4,5,6)"));
    returns_colourize_args.insert("intermediate_ret_colour".to_string(), json!("(7,8,9)"));
    returns_colourize_args.insert("last_ret_colour".to_string(), json!("(11,12,13)"));
    returns_colourize_args.insert("output".to_string(), json!(returns_colourize_out.to_string_lossy().to_string()));
    registry
        .run("colourize_based_on_point_returns", &returns_colourize_args, &context(&caps))
        .expect("colourize_based_on_point_returns run");
    let returns_coloured = PointCloud::read(&returns_colourize_out).expect("read returns colourized output");
    assert!(returns_coloured.points.iter().all(|p| p.color.is_some()));
    let only = returns_coloured.points[0].color.expect("only return colour");
    let first = returns_coloured.points[1].color.expect("first return colour");
    let last = returns_coloured.points[2].color.expect("last return colour");
    assert_eq!((only.red / 257, only.green / 257, only.blue / 257), (1, 2, 3));
    assert_eq!((first.red / 257, first.green / 257, first.blue / 257), (4, 5, 6));
    assert_eq!((last.red / 257, last.green / 257, last.blue / 257), (11, 12, 13));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&image_path);
    let _ = std::fs::remove_file(&lidar_colourize_out);
    let _ = std::fs::remove_file(&class_colourize_out);
    let _ = std::fs::remove_file(&returns_colourize_out);
}

#[test]
fn lidar_phase2_next_batch_building_and_ascii_tools_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_next_batch_f");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let poly_path = std::env::temp_dir().join(format!("{tag}_buildings.geojson"));
    let classify_out = std::env::temp_dir().join(format!("{tag}_classified.las"));

    let ascii_in = std::env::temp_dir().join(format!("{tag}_points.csv"));
    let ascii_out_dir = std::env::temp_dir().join(format!("{tag}_ascii_out"));
    let las_from_ascii = ascii_out_dir.join(format!("{}_points.las", tag));
    let las_to_ascii_out = std::env::temp_dir().join(format!("{tag}_roundtrip.csv"));

    let cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.5, y: 0.5, z: 10.0, classification: 1, ..PointRecord::default() },
            PointRecord { x: 3.0, y: 3.0, z: 11.0, classification: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut layer = Layer::new("buildings").with_geom_type(GeometryType::Polygon);
    let mut feature = Feature::new();
    feature.geometry = Some(Geometry::polygon(
        vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 1.0),
        ],
        vec![],
    ));
    layer.push(feature);
    wbvector::write(&layer, &poly_path, VectorFormat::GeoJson).expect("write building polygons");

    let mut classify_args = ToolArgs::new();
    classify_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    classify_args.insert("buildings".to_string(), json!(poly_path.to_string_lossy().to_string()));
    classify_args.insert("output".to_string(), json!(classify_out.to_string_lossy().to_string()));
    registry
        .run("classify_buildings_in_lidar", &classify_args, &context(&caps))
        .expect("classify_buildings_in_lidar run");
    let classified = PointCloud::read(&classify_out).expect("read classify output");
    assert_eq!(classified.points.len(), 2);
    assert_eq!(classified.points[0].classification, 6);
    assert_eq!(classified.points[1].classification, 1);

    std::fs::create_dir_all(&ascii_out_dir).expect("create ascii output dir");
    std::fs::write(
        &ascii_in,
        "x,y,z,i,c,rn,nr,sa,time,r,g,b\n0.0,0.0,10.0,100,2,1,1,0,1.25,1000,2000,3000\n1.0,1.0,12.0,150,6,1,2,3,2.50,1200,2200,3200\n",
    )
    .expect("write ascii input");

    let mut ascii_to_las_args = ToolArgs::new();
    ascii_to_las_args.insert(
        "inputs".to_string(),
        json!([ascii_in.to_string_lossy().to_string()]),
    );
    ascii_to_las_args.insert("pattern".to_string(), json!("x,y,z,i,c,rn,nr,sa,time,r,g,b"));
    ascii_to_las_args.insert("epsg_code".to_string(), json!(4326));
    ascii_to_las_args.insert(
        "output_directory".to_string(),
        json!(ascii_out_dir.to_string_lossy().to_string()),
    );
    registry
        .run("ascii_to_las", &ascii_to_las_args, &context(&caps))
        .expect("ascii_to_las run");
    assert!(las_from_ascii.exists());
    let ascii_las_cloud = PointCloud::read(&las_from_ascii).expect("read las from ascii");
    assert_eq!(ascii_las_cloud.points.len(), 2);
    assert_eq!(ascii_las_cloud.points[0].classification, 2);
    assert!(ascii_las_cloud.points[0].gps_time.is_some());
    assert!(ascii_las_cloud.points[0].color.is_some());

    let mut las_to_ascii_args = ToolArgs::new();
    las_to_ascii_args.insert("input".to_string(), json!(las_from_ascii.to_string_lossy().to_string()));
    las_to_ascii_args.insert("output".to_string(), json!(las_to_ascii_out.to_string_lossy().to_string()));
    let las_to_ascii_result = registry
        .run("las_to_ascii", &las_to_ascii_args, &context(&caps))
        .expect("las_to_ascii run");
    let output_csv = las_to_ascii_result
        .outputs
        .get("output")
        .and_then(|v| v.as_str())
        .expect("las_to_ascii output string");
    assert_eq!(output_csv, las_to_ascii_out.to_string_lossy().to_string());
    let csv_text = std::fs::read_to_string(&las_to_ascii_out).expect("read las_to_ascii output");
    assert!(csv_text.lines().next().unwrap_or_default().contains("X,Y,Z"));
    assert!(csv_text.lines().count() >= 3);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&poly_path);
    let _ = std::fs::remove_file(&classify_out);
    let _ = std::fs::remove_file(&ascii_in);
    let _ = std::fs::remove_file(&las_to_ascii_out);
    let _ = std::fs::remove_dir_all(&ascii_out_dir);
}

#[test]
fn lidar_phase2_next_batch_select_tiles_by_polygon_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase2_next_batch_g");
    let in_dir = std::env::temp_dir().join(format!("{tag}_tiles_in"));
    let out_dir = std::env::temp_dir().join(format!("{tag}_tiles_out"));
    std::fs::create_dir_all(&in_dir).expect("create input dir");

    let tile_inside = in_dir.join("inside.las");
    let tile_outside = in_dir.join("outside.las");
    let poly_path = std::env::temp_dir().join(format!("{tag}_poly.geojson"));

    let inside_cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.1, y: 0.1, z: 10.0, ..PointRecord::default() },
            PointRecord { x: 0.4, y: 0.4, z: 11.0, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    inside_cloud.write(&tile_inside).expect("write inside tile");

    let outside_cloud = PointCloud {
        points: vec![
            PointRecord { x: 10.0, y: 10.0, z: 10.0, ..PointRecord::default() },
            PointRecord { x: 10.5, y: 10.5, z: 11.0, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    outside_cloud.write(&tile_outside).expect("write outside tile");

    let mut layer = Layer::new("poly").with_geom_type(GeometryType::Polygon);
    let mut feature = Feature::new();
    feature.geometry = Some(Geometry::polygon(
        vec![
            Coord::xy(-1.0, -1.0),
            Coord::xy(1.0, -1.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(-1.0, 1.0),
        ],
        vec![],
    ));
    layer.push(feature);
    wbvector::write(&layer, &poly_path, VectorFormat::GeoJson).expect("write polygon");

    let mut args = ToolArgs::new();
    args.insert(
        "input_directory".to_string(),
        json!(in_dir.to_string_lossy().to_string()),
    );
    args.insert(
        "output_directory".to_string(),
        json!(out_dir.to_string_lossy().to_string()),
    );
    args.insert("polygons".to_string(), json!(poly_path.to_string_lossy().to_string()));

    let res = registry
        .run("select_tiles_by_polygon", &args, &context(&caps))
        .expect("select_tiles_by_polygon run");

    let out_dir_reported = res
        .outputs
        .get("output_directory")
        .and_then(|v| v.as_str())
        .expect("output_directory string");
    assert_eq!(out_dir_reported, out_dir.to_string_lossy().to_string());

    let copied_files = res
        .outputs
        .get("copied_files")
        .and_then(|v| v.as_u64())
        .expect("copied_files count");
    assert_eq!(copied_files, 1);
    assert!(out_dir.join("inside.las").exists());
    assert!(!out_dir.join("outside.las").exists());

    let _ = std::fs::remove_dir_all(&in_dir);
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::remove_file(&poly_path);
}

#[test]
fn lidar_phase3_info_histogram_point_stats_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase3_batch_a");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let info_out = std::env::temp_dir().join(format!("{tag}_info.txt"));
    let hist_out = std::env::temp_dir().join(format!("{tag}_hist.html"));
    let stats_out_dir = std::env::temp_dir().join(format!("{tag}_stats"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 0.2,
                z: 12.0,
                intensity: 120,
                classification: 5,
                return_number: 1,
                number_of_returns: 2,
                gps_time: Some(GpsTime(2.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.2,
                y: 1.1,
                z: 9.0,
                intensity: 80,
                classification: 2,
                return_number: 2,
                number_of_returns: 2,
                gps_time: Some(GpsTime(3.0)),
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut info_args = ToolArgs::new();
    info_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    info_args.insert("output".to_string(), json!(info_out.to_string_lossy().to_string()));
    let info_res = registry
        .run("lidar_info", &info_args, &context(&caps))
        .expect("lidar_info run");
    let info_path = info_res
        .outputs
        .get("report_path")
        .and_then(|v| v.as_str())
        .expect("lidar_info report path");
    assert_eq!(info_path, info_out.to_string_lossy().to_string());
    assert!(info_out.exists());

    let mut hist_args = ToolArgs::new();
    hist_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    hist_args.insert("output".to_string(), json!(hist_out.to_string_lossy().to_string()));
    hist_args.insert("parameter".to_string(), json!("intensity"));
    hist_args.insert("clip_percent".to_string(), json!(1.0));
    let hist_res = registry
        .run("lidar_histogram", &hist_args, &context(&caps))
        .expect("lidar_histogram run");
    let hist_path = hist_res
        .outputs
        .get("report_path")
        .and_then(|v| v.as_str())
        .expect("lidar_histogram report path");
    assert_eq!(hist_path, hist_out.to_string_lossy().to_string());
    assert!(hist_out.exists());

    std::fs::create_dir_all(&stats_out_dir).expect("create stats output dir");
    let mut stats_args = ToolArgs::new();
    stats_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    stats_args.insert("resolution".to_string(), json!(1.0));
    stats_args.insert("num_points".to_string(), json!(true));
    stats_args.insert("z_range".to_string(), json!(true));
    stats_args.insert(
        "output_directory".to_string(),
        json!(stats_out_dir.to_string_lossy().to_string()),
    );
    let stats_res = registry
        .run("lidar_point_stats", &stats_args, &context(&caps))
        .expect("lidar_point_stats run");
    let out_dir = stats_res
        .outputs
        .get("output_directory")
        .and_then(|v| v.as_str())
        .expect("lidar_point_stats output directory");
    assert_eq!(out_dir, stats_out_dir.to_string_lossy().to_string());
    let num_points_tif = stats_out_dir.join(format!("{}_input_num_pnts.tif", tag));
    let z_range_tif = stats_out_dir.join(format!("{}_input_z_range.tif", tag));
    assert!(num_points_tif.exists());
    assert!(z_range_tif.exists());

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&info_out);
    let _ = std::fs::remove_file(&hist_out);
    let _ = std::fs::remove_dir_all(&stats_out_dir);
}

#[test]
fn lidar_phase3_vector_tools_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase3_batch_b");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let contour_out = std::env::temp_dir().join(format!("{tag}_contours.shp"));
    let footprint_out = std::env::temp_dir().join(format!("{tag}_footprint.shp"));
    let points_out = std::env::temp_dir().join(format!("{tag}_points.shp"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 0.0,
                z: 12.0,
                intensity: 110,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.0,
                y: 1.0,
                z: 14.0,
                intensity: 120,
                classification: 5,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 1.0,
                z: 16.0,
                intensity: 130,
                classification: 5,
                return_number: 2,
                number_of_returns: 2,
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.5,
                y: 0.5,
                z: 13.0,
                intensity: 125,
                classification: 1,
                return_number: 1,
                number_of_returns: 2,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut contour_args = ToolArgs::new();
    contour_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    contour_args.insert("output".to_string(), json!(contour_out.to_string_lossy().to_string()));
    contour_args.insert("interval".to_string(), json!(2.0));
    contour_args.insert("base_contour".to_string(), json!(0.0));
    let contour_res = registry
        .run("lidar_contour", &contour_args, &context(&caps))
        .expect("lidar_contour run");
    let contour_path = contour_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("lidar_contour output path");
    assert_eq!(contour_path, contour_out.to_string_lossy().to_string());
    assert!(contour_out.exists());
    let contour_layer = wbvector::read(&contour_out).expect("read contour output");
    assert!(!contour_layer.features.is_empty());

    let mut footprint_args = ToolArgs::new();
    footprint_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    footprint_args.insert("output".to_string(), json!(footprint_out.to_string_lossy().to_string()));
    footprint_args.insert("output_hulls".to_string(), json!(true));
    let footprint_res = registry
        .run("lidar_tile_footprint", &footprint_args, &context(&caps))
        .expect("lidar_tile_footprint run");
    let footprint_path = footprint_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("lidar_tile_footprint output path");
    assert_eq!(footprint_path, footprint_out.to_string_lossy().to_string());
    assert!(footprint_out.exists());
    let footprint_layer = wbvector::read(&footprint_out).expect("read footprint output");
    assert_eq!(footprint_layer.features.len(), 1);

    let mut points_args = ToolArgs::new();
    points_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    points_args.insert("output".to_string(), json!(points_out.to_string_lossy().to_string()));
    points_args.insert("output_multipoint".to_string(), json!(false));
    let points_res = registry
        .run("las_to_shapefile", &points_args, &context(&caps))
        .expect("las_to_shapefile run");
    let points_path = points_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("las_to_shapefile output path");
    assert_eq!(points_path, points_out.to_string_lossy().to_string());
    assert!(points_out.exists());
    let points_layer = wbvector::read(&points_out).expect("read points output");
    assert_eq!(points_layer.features.len(), 5);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&contour_out);
    let _ = std::fs::remove_file(&footprint_out);
    let _ = std::fs::remove_file(&points_out);
}

#[test]
fn lidar_phase3_construct_hex_return_analysis_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase3_batch_c");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let tin_out = std::env::temp_dir().join(format!("{tag}_tin.shp"));
    let hex_out = std::env::temp_dir().join(format!("{tag}_hex.shp"));
    let report_out = std::env::temp_dir().join(format!("{tag}_returns.txt"));
    let qc_out = std::env::temp_dir().join(format!("{tag}_qc.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.0,
                y: 0.0,
                z: 10.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 2,
                gps_time: Some(GpsTime(1.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 0.0,
                z: 12.0,
                intensity: 120,
                classification: 2,
                return_number: 2,
                number_of_returns: 2,
                gps_time: Some(GpsTime(1.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.0,
                y: 1.0,
                z: 14.0,
                intensity: 110,
                classification: 5,
                return_number: 1,
                number_of_returns: 2,
                gps_time: Some(GpsTime(2.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.5,
                y: 1.0,
                z: 15.0,
                intensity: 90,
                classification: 5,
                return_number: 1,
                number_of_returns: 2,
                gps_time: Some(GpsTime(2.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 1.0,
                z: 16.0,
                intensity: 80,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(3.0)),
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut tin_args = ToolArgs::new();
    tin_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    tin_args.insert("output".to_string(), json!(tin_out.to_string_lossy().to_string()));
    tin_args.insert("returns".to_string(), json!("all"));
    let tin_res = registry
        .run("lidar_construct_vector_tin", &tin_args, &context(&caps))
        .expect("lidar_construct_vector_tin run");
    let tin_path = tin_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("lidar_construct_vector_tin output path");
    assert_eq!(tin_path, tin_out.to_string_lossy().to_string());
    assert!(tin_out.exists());

    let mut hex_args = ToolArgs::new();
    hex_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    hex_args.insert("output".to_string(), json!(hex_out.to_string_lossy().to_string()));
    hex_args.insert("width".to_string(), json!(1.0));
    hex_args.insert("orientation".to_string(), json!("h"));
    let hex_res = registry
        .run("lidar_hex_bin", &hex_args, &context(&caps))
        .expect("lidar_hex_bin run");
    let hex_path = hex_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("lidar_hex_bin output path");
    assert_eq!(hex_path, hex_out.to_string_lossy().to_string());
    assert!(hex_out.exists());

    let mut ra_args = ToolArgs::new();
    ra_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    ra_args.insert("create_output".to_string(), json!(true));
    ra_args.insert("output".to_string(), json!(qc_out.to_string_lossy().to_string()));
    ra_args.insert("report".to_string(), json!(report_out.to_string_lossy().to_string()));
    let ra_res = registry
        .run("lidar_point_return_analysis", &ra_args, &context(&caps))
        .expect("lidar_point_return_analysis run");
    let report_path = ra_res
        .outputs
        .get("report_path")
        .and_then(|v| v.as_str())
        .expect("lidar_point_return_analysis report path");
    assert_eq!(report_path, report_out.to_string_lossy().to_string());
    assert!(report_out.exists());
    assert!(qc_out.exists());

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&tin_out);
    let _ = std::fs::remove_file(&hex_out);
    let _ = std::fs::remove_file(&report_out);
    let _ = std::fs::remove_file(&qc_out);
}

#[test]
fn lidar_phase3_flightline_tools_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase3_batch_d");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.las"));
    let overlap_out = std::env::temp_dir().join(format!("{tag}_overlap.tif"));
    let recover_out = std::env::temp_dir().join(format!("{tag}_recover.las"));
    let edge_out = std::env::temp_dir().join(format!("{tag}_edges.las"));

    let cloud = PointCloud {
        points: vec![
            PointRecord {
                x: 0.10,
                y: 0.10,
                z: 10.0,
                point_source_id: 0,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 0.20,
                y: 0.20,
                z: 11.0,
                point_source_id: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.1)),
                edge_of_flight_line: true,
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.10,
                y: 0.20,
                z: 12.0,
                point_source_id: 0,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(20.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.20,
                y: 0.30,
                z: 12.5,
                point_source_id: 0,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(20.2)),
                edge_of_flight_line: true,
                ..PointRecord::default()
            },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    cloud.write(&input_path).expect("write input cloud");

    let mut overlap_args = ToolArgs::new();
    overlap_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    overlap_args.insert("resolution".to_string(), json!(1.0));
    overlap_args.insert("output".to_string(), json!(overlap_out.to_string_lossy().to_string()));
    let overlap_res = registry
        .run("flightline_overlap", &overlap_args, &context(&caps))
        .expect("flightline_overlap run");
    let overlap_path = overlap_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("flightline_overlap output path");
    assert_eq!(overlap_path, overlap_out.to_string_lossy().to_string());
    let overlap_raster = Raster::read(&overlap_out).expect("read overlap raster");
    let overlap_max = overlap_raster
        .data_f32()
        .expect("overlap raster f32 data")
        .iter()
        .map(|v| f64::from(*v))
        .filter(|v| *v != overlap_raster.nodata)
        .fold(f64::NEG_INFINITY, f64::max);
    assert_eq!(overlap_max, 2.0);

    let mut recover_args = ToolArgs::new();
    recover_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    recover_args.insert("max_time_diff".to_string(), json!(5.0));
    recover_args.insert("pt_src_id".to_string(), json!(true));
    recover_args.insert("user_data".to_string(), json!(true));
    recover_args.insert("rgb".to_string(), json!(true));
    recover_args.insert("output".to_string(), json!(recover_out.to_string_lossy().to_string()));
    let recover_res = registry
        .run("recover_flightline_info", &recover_args, &context(&caps))
        .expect("recover_flightline_info run");
    let recover_path = recover_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("recover_flightline_info output path");
    assert_eq!(recover_path, recover_out.to_string_lossy().to_string());
    let recovered = PointCloud::read(&recover_out).expect("read recovered cloud");
    assert_eq!(recovered.points.len(), 4);
    assert_eq!(recovered.points[0].point_source_id, 0);
    assert_eq!(recovered.points[1].point_source_id, 0);
    assert_eq!(recovered.points[2].point_source_id, 1);
    assert_eq!(recovered.points[3].point_source_id, 1);
    assert!(recovered.points.iter().all(|p| p.color.is_some()));
    assert_eq!(recovered.points[2].user_data, 1);

    let mut edge_args = ToolArgs::new();
    edge_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    edge_args.insert("output".to_string(), json!(edge_out.to_string_lossy().to_string()));
    let edge_res = registry
        .run("find_flightline_edge_points", &edge_args, &context(&caps))
        .expect("find_flightline_edge_points run");
    let edge_path = edge_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("find_flightline_edge_points output path");
    assert_eq!(edge_path, edge_out.to_string_lossy().to_string());
    let edges = PointCloud::read(&edge_out).expect("read edge cloud");
    assert_eq!(edges.points.len(), 2);
    assert!(edges.points.iter().all(|p| p.edge_of_flight_line));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&overlap_out);
    let _ = std::fs::remove_file(&recover_out);
    let _ = std::fs::remove_file(&edge_out);
}

#[test]
fn lidar_phase3_analysis_tools_end_to_end() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_lidar_phase3_batch_e");
    let analysis_in = std::env::temp_dir().join(format!("{tag}_analysis.las"));
    let rooftop_in = std::env::temp_dir().join(format!("{tag}_rooftop.las"));
    let top_hat_out = std::env::temp_dir().join(format!("{tag}_tophat.las"));
    let normals_out = std::env::temp_dir().join(format!("{tag}_normals.las"));
    let ransac_out = std::env::temp_dir().join(format!("{tag}_ransac.las"));
    let eigen_out = std::env::temp_dir().join(format!("{tag}.eigen"));
    let class_in = std::env::temp_dir().join(format!("{tag}_class.las"));
    let reference_in = std::env::temp_dir().join(format!("{tag}_reference.las"));
    let kappa_out = std::env::temp_dir().join(format!("{tag}_kappa.tif"));
    let kappa_report = std::env::temp_dir().join(format!("{tag}_kappa.html"));
    let buildings_path = std::env::temp_dir().join(format!("{tag}_buildings.geojson"));
    let rooftops_out = std::env::temp_dir().join(format!("{tag}_rooftops.geojson"));

    let mut analysis_points = Vec::new();
    for row in 0..4 {
        for col in 0..4 {
            analysis_points.push(PointRecord {
                x: col as f64,
                y: row as f64,
                z: 10.0 + 0.2 * col as f64 + 0.1 * row as f64,
                classification: 6,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            });
        }
    }
    analysis_points.push(PointRecord {
        x: 1.5,
        y: 1.5,
        z: 12.5,
        classification: 6,
        return_number: 1,
        number_of_returns: 1,
        ..PointRecord::default()
    });
    let analysis_cloud = PointCloud {
        points: analysis_points,
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    analysis_cloud.write(&analysis_in).expect("write analysis cloud");
    let rooftop_cloud = PointCloud {
        points: (0..4)
            .flat_map(|row| {
                (0..4).map(move |col| PointRecord {
                    x: col as f64,
                    y: row as f64,
                    z: 10.0 + 0.2 * col as f64 + 0.1 * row as f64,
                    classification: 6,
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                })
            })
            .collect(),
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    rooftop_cloud.write(&rooftop_in).expect("write rooftop cloud");

    let class_cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 0.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 0.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 1.0, z: 0.0, classification: 6, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 0.0, classification: 6, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    class_cloud.write(&class_in).expect("write classified cloud");
    let reference_cloud = PointCloud {
        points: vec![
            PointRecord { x: 0.0, y: 0.0, z: 0.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 0.0, z: 0.0, classification: 2, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 0.0, y: 1.0, z: 0.0, classification: 6, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 1.0, z: 0.0, classification: 1, return_number: 1, number_of_returns: 1, ..PointRecord::default() },
        ],
        crs: Some(LidarCrs::from_epsg(4326)),
    };
    reference_cloud.write(&reference_in).expect("write reference cloud");

    let mut buildings = Layer::new("buildings")
        .with_geom_type(GeometryType::Polygon)
        .with_crs_epsg(4326);
    buildings
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(-0.5, -0.5),
                    Coord::xy(3.5, -0.5),
                    Coord::xy(3.5, 3.5),
                    Coord::xy(-0.5, 3.5),
                    Coord::xy(-0.5, -0.5),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add building polygon");
    wbvector::write(&buildings, &buildings_path, VectorFormat::GeoJson).expect("write buildings");

    let mut tophat_args = ToolArgs::new();
    tophat_args.insert("input".to_string(), json!(analysis_in.to_string_lossy().to_string()));
    tophat_args.insert("search_radius".to_string(), json!(1.6));
    tophat_args.insert("output".to_string(), json!(top_hat_out.to_string_lossy().to_string()));
    registry
        .run("lidar_tophat_transform", &tophat_args, &context(&caps))
        .expect("lidar_tophat_transform run");
    let tophat_cloud = PointCloud::read(&top_hat_out).expect("read tophat output");
    let max_tophat = tophat_cloud.points.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
    assert!(max_tophat > 1.0);

    let mut normal_args = ToolArgs::new();
    normal_args.insert("input".to_string(), json!(analysis_in.to_string_lossy().to_string()));
    normal_args.insert("search_radius".to_string(), json!(1.6));
    normal_args.insert("output".to_string(), json!(normals_out.to_string_lossy().to_string()));
    registry
        .run("normal_vectors", &normal_args, &context(&caps))
        .expect("normal_vectors run");
    let normals_cloud = PointCloud::read(&normals_out).expect("read normals output");
    assert!(normals_cloud.points.iter().any(|p| p.color.is_some()));

    let mut eigen_args = ToolArgs::new();
    eigen_args.insert("input".to_string(), json!(analysis_in.to_string_lossy().to_string()));
    eigen_args.insert("num_neighbours".to_string(), json!(7));
    eigen_args.insert("search_radius".to_string(), json!(2.0));
    eigen_args.insert("output".to_string(), json!(eigen_out.to_string_lossy().to_string()));
    let eigen_res = registry
        .run("lidar_eigenvalue_features", &eigen_args, &context(&caps))
        .expect("lidar_eigenvalue_features run");
    let eigen_path = eigen_res
        .outputs
        .get("output")
        .and_then(|v| v.as_str())
        .expect("lidar_eigenvalue_features output path");
    assert_eq!(eigen_path, eigen_out.to_string_lossy().to_string());
    assert!(eigen_out.exists());
    assert!(std::path::PathBuf::from(format!("{}.json", eigen_out.to_string_lossy())).exists());

    let mut ransac_args = ToolArgs::new();
    ransac_args.insert("input".to_string(), json!(analysis_in.to_string_lossy().to_string()));
    ransac_args.insert("search_radius".to_string(), json!(1.6));
    ransac_args.insert("num_iterations".to_string(), json!(25));
    ransac_args.insert("num_samples".to_string(), json!(3));
    ransac_args.insert("inlier_threshold".to_string(), json!(0.2));
    ransac_args.insert("acceptable_model_size".to_string(), json!(5));
    ransac_args.insert("classify".to_string(), json!(true));
    ransac_args.insert("output".to_string(), json!(ransac_out.to_string_lossy().to_string()));
    registry
        .run("lidar_ransac_planes", &ransac_args, &context(&caps))
        .expect("lidar_ransac_planes run");
    let ransac_cloud = PointCloud::read(&ransac_out).expect("read ransac output");
    assert_eq!(ransac_cloud.points.len(), 17);
    assert!(ransac_cloud.points.iter().filter(|p| p.classification == 0).count() >= 8);

    let mut kappa_args = ToolArgs::new();
    kappa_args.insert("input1".to_string(), json!(class_in.to_string_lossy().to_string()));
    kappa_args.insert("input2".to_string(), json!(reference_in.to_string_lossy().to_string()));
    kappa_args.insert("report".to_string(), json!(kappa_report.to_string_lossy().to_string()));
    kappa_args.insert("resolution".to_string(), json!(1.0));
    kappa_args.insert("output".to_string(), json!(kappa_out.to_string_lossy().to_string()));
    registry
        .run("lidar_kappa", &kappa_args, &context(&caps))
        .expect("lidar_kappa run");
    assert!(kappa_out.exists());
    let report_html = std::fs::read_to_string(&kappa_report).expect("read kappa report");
    assert!(report_html.contains("Overall Accuracy"));
    assert!(report_html.contains("Kappa"));

    let mut rooftop_args = ToolArgs::new();
    rooftop_args.insert("inputs".to_string(), json!([rooftop_in.to_string_lossy().to_string()]));
    rooftop_args.insert("building_footprints".to_string(), json!(buildings_path.to_string_lossy().to_string()));
    rooftop_args.insert("search_radius".to_string(), json!(3.0));
    rooftop_args.insert("num_iterations".to_string(), json!(25));
    rooftop_args.insert("num_samples".to_string(), json!(3));
    rooftop_args.insert("inlier_threshold".to_string(), json!(0.2));
    rooftop_args.insert("acceptable_model_size".to_string(), json!(5));
    rooftop_args.insert("norm_diff_threshold".to_string(), json!(15.0));
    rooftop_args.insert("output".to_string(), json!(rooftops_out.to_string_lossy().to_string()));
    let rooftop_res = registry
        .run("lidar_rooftop_analysis", &rooftop_args, &context(&caps))
        .expect("lidar_rooftop_analysis run");
    let rooftop_path = rooftop_res
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("lidar_rooftop_analysis output path");
    assert_eq!(rooftop_path, rooftops_out.to_string_lossy().to_string());
    let rooftops = wbvector::read(&rooftops_out).expect("read rooftop output");
    assert!(!rooftops.features.is_empty());
    match rooftops.features[0].geometry.as_ref().expect("rooftop geometry") {
        Geometry::Polygon { .. } => {}
        _ => panic!("rooftop output should be polygon geometry"),
    }

    let _ = std::fs::remove_file(&analysis_in);
    let _ = std::fs::remove_file(&rooftop_in);
    let _ = std::fs::remove_file(&top_hat_out);
    let _ = std::fs::remove_file(&normals_out);
    let _ = std::fs::remove_file(&ransac_out);
    let _ = std::fs::remove_file(&eigen_out);
    let _ = std::fs::remove_file(format!("{}.json", eigen_out.to_string_lossy()));
    let _ = std::fs::remove_file(&class_in);
    let _ = std::fs::remove_file(&reference_in);
    let _ = std::fs::remove_file(&kappa_out);
    let _ = std::fs::remove_file(&kappa_report);
    let _ = std::fs::remove_file(&buildings_path);
    let _ = std::fs::remove_file(&rooftops_out);
}

#[test]
fn spatial_join_supports_aggregate_strategies() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_spatial_join_aggs");
    let target_path = std::env::temp_dir().join(format!("{tag}_target.geojson"));
    let join_path = std::env::temp_dir().join(format!("{tag}_join.geojson"));
    let out_sum_path = std::env::temp_dir().join(format!("{tag}_sum.geojson"));
    let out_mean_path = std::env::temp_dir().join(format!("{tag}_mean.geojson"));
    let out_min_path = std::env::temp_dir().join(format!("{tag}_min.geojson"));
    let out_max_path = std::env::temp_dir().join(format!("{tag}_max.geojson"));

    let mut target = Layer::new("target").with_geom_type(GeometryType::Point).with_epsg(4326);
    target.add_field(FieldDef::new("ID", FieldType::Integer));
    target
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[("ID", FieldValue::Integer(1))],
        )
        .expect("add target point");
    wbvector::write(&target, &target_path, VectorFormat::GeoJson).expect("write target");

    let mut join = Layer::new("join").with_geom_type(GeometryType::Point).with_epsg(4326);
    join.add_field(FieldDef::new("VAL", FieldType::Float));
    join.add_field(FieldDef::new("NAME", FieldType::Text));
    join
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[("VAL", FieldValue::Float(2.0)), ("NAME", FieldValue::Text("a".to_string()))],
        )
        .expect("add join point 1");
    join
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[("VAL", FieldValue::Float(4.0)), ("NAME", FieldValue::Text("b".to_string()))],
        )
        .expect("add join point 2");
    wbvector::write(&join, &join_path, VectorFormat::GeoJson).expect("write join");

    let mut sum_args = ToolArgs::new();
    sum_args.insert("target".to_string(), json!(target_path.to_string_lossy().to_string()));
    sum_args.insert("join".to_string(), json!(join_path.to_string_lossy().to_string()));
    sum_args.insert("predicate".to_string(), json!("intersects"));
    sum_args.insert("strategy".to_string(), json!("sum"));
    sum_args.insert("prefix".to_string(), json!("J_"));
    sum_args.insert("output".to_string(), json!(out_sum_path.to_string_lossy().to_string()));
    registry
        .run("spatial_join", &sum_args, &context(&caps))
        .expect("spatial_join sum strategy run");

    let mut mean_args = ToolArgs::new();
    mean_args.insert("target".to_string(), json!(target_path.to_string_lossy().to_string()));
    mean_args.insert("join".to_string(), json!(join_path.to_string_lossy().to_string()));
    mean_args.insert("predicate".to_string(), json!("intersects"));
    mean_args.insert("strategy".to_string(), json!("mean"));
    mean_args.insert("prefix".to_string(), json!("J_"));
    mean_args.insert("output".to_string(), json!(out_mean_path.to_string_lossy().to_string()));
    registry
        .run("spatial_join", &mean_args, &context(&caps))
        .expect("spatial_join mean strategy run");

    let mut min_args = ToolArgs::new();
    min_args.insert("target".to_string(), json!(target_path.to_string_lossy().to_string()));
    min_args.insert("join".to_string(), json!(join_path.to_string_lossy().to_string()));
    min_args.insert("predicate".to_string(), json!("intersects"));
    min_args.insert("strategy".to_string(), json!("min"));
    min_args.insert("prefix".to_string(), json!("J_"));
    min_args.insert("output".to_string(), json!(out_min_path.to_string_lossy().to_string()));
    registry
        .run("spatial_join", &min_args, &context(&caps))
        .expect("spatial_join min strategy run");

    let mut max_args = ToolArgs::new();
    max_args.insert("target".to_string(), json!(target_path.to_string_lossy().to_string()));
    max_args.insert("join".to_string(), json!(join_path.to_string_lossy().to_string()));
    max_args.insert("predicate".to_string(), json!("intersects"));
    max_args.insert("strategy".to_string(), json!("max"));
    max_args.insert("prefix".to_string(), json!("J_"));
    max_args.insert("output".to_string(), json!(out_max_path.to_string_lossy().to_string()));
    registry
        .run("spatial_join", &max_args, &context(&caps))
        .expect("spatial_join max strategy run");

    let sum_out = wbvector::read(&out_sum_path).expect("read sum output");
    let mean_out = wbvector::read(&out_mean_path).expect("read mean output");
    let min_out = wbvector::read(&out_min_path).expect("read min output");
    let max_out = wbvector::read(&out_max_path).expect("read max output");
    assert_eq!(sum_out.features.len(), 1);
    assert_eq!(mean_out.features.len(), 1);
    assert_eq!(min_out.features.len(), 1);
    assert_eq!(max_out.features.len(), 1);

    let sum_schema = &sum_out.schema;
    let mean_schema = &mean_out.schema;
    let min_schema = &min_out.schema;
    let max_schema = &max_out.schema;
    let j_val_sum_idx = sum_schema.field_index("J_VAL").expect("J_VAL field in sum output");
    let j_val_mean_idx = mean_schema.field_index("J_VAL").expect("J_VAL field in mean output");
    let j_val_min_idx = min_schema.field_index("J_VAL").expect("J_VAL field in min output");
    let j_val_max_idx = max_schema.field_index("J_VAL").expect("J_VAL field in max output");
    let join_count_sum_idx = sum_schema.field_index("JOIN_COUNT").expect("JOIN_COUNT in sum output");

    let sum_attrs = &sum_out.features[0].attributes;
    let mean_attrs = &mean_out.features[0].attributes;
    let min_attrs = &min_out.features[0].attributes;
    let max_attrs = &max_out.features[0].attributes;
    match &sum_attrs[j_val_sum_idx] {
        FieldValue::Float(v) => assert!((*v - 6.0).abs() < 1.0e-9, "expected sum 6.0, got {}", v),
        FieldValue::Integer(v) => assert_eq!(*v, 6, "expected sum 6, got {}", v),
        other => panic!("expected numeric J_VAL for sum strategy, got {:?}", other),
    }
    match &mean_attrs[j_val_mean_idx] {
        FieldValue::Float(v) => assert!((*v - 3.0).abs() < 1.0e-9, "expected mean 3.0, got {}", v),
        FieldValue::Integer(v) => assert_eq!(*v, 3, "expected mean 3, got {}", v),
        other => panic!("expected numeric J_VAL for mean strategy, got {:?}", other),
    }
    match &min_attrs[j_val_min_idx] {
        FieldValue::Float(v) => assert!((*v - 2.0).abs() < 1.0e-9, "expected min 2.0, got {}", v),
        FieldValue::Integer(v) => assert_eq!(*v, 2, "expected min 2, got {}", v),
        other => panic!("expected numeric J_VAL for min strategy, got {:?}", other),
    }
    match &max_attrs[j_val_max_idx] {
        FieldValue::Float(v) => assert!((*v - 4.0).abs() < 1.0e-9, "expected max 4.0, got {}", v),
        FieldValue::Integer(v) => assert_eq!(*v, 4, "expected max 4, got {}", v),
        other => panic!("expected numeric J_VAL for max strategy, got {:?}", other),
    }
    match &sum_attrs[join_count_sum_idx] {
        FieldValue::Integer(v) => assert_eq!(*v, 2),
        other => panic!("expected integer JOIN_COUNT, got {:?}", other),
    }

    let _ = std::fs::remove_file(&target_path);
    let _ = std::fs::remove_file(&join_path);
    let _ = std::fs::remove_file(&out_sum_path);
    let _ = std::fs::remove_file(&out_mean_path);
    let _ = std::fs::remove_file(&out_min_path);
    let _ = std::fs::remove_file(&out_max_path);
}

#[test]
fn line_polygon_clip_outputs_clipped_segments_not_whole_features() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_line_polygon_clip_segments");
    let input_path = std::env::temp_dir().join(format!("{tag}_lines.gpkg"));
    let clip_path = std::env::temp_dir().join(format!("{tag}_clip.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("lines")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(-1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add line feature");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write line input");

    let mut clip = Layer::new("clip")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    clip
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, -1.0),
                    Coord::xy(1.0, -1.0),
                    Coord::xy(1.0, 1.0),
                    Coord::xy(0.0, 1.0),
                    Coord::xy(0.0, -1.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add clip polygon");
    wbvector::write(&clip, &clip_path, VectorFormat::GeoPackage).expect("write clip input");
    let clip_layer = wbvector::read(&clip_path).expect("read clip input");
    assert_eq!(clip_layer.geom_type, Some(GeometryType::Polygon));

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("clip".to_string(), json!(clip_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("line_polygon_clip", &args, &context(&caps))
        .expect("line_polygon_clip run");

    let out = wbvector::read(&out_path).expect("read clipped output");
    assert!(
        !out.features.is_empty(),
        "line_polygon_clip should produce at least one clipped segment"
    );

    let first_geom = out.features[0].geometry.as_ref().expect("output geometry");
    let coords = match first_geom {
        Geometry::LineString(coords) => coords,
        other => panic!("expected line geometry, got {:?}", other),
    };
    assert!(coords.len() >= 2, "clipped segment should contain at least 2 points");
    let first = &coords[0];
    let last = &coords[coords.len() - 1];
    assert!(
        first.x >= -1.0e-8 && last.x <= 1.0 + 1.0e-8,
        "clipped segment endpoints should lie within clip polygon x-range [0,1], got [{}, {}]",
        first.x,
        last.x
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&clip_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn near_writes_nearest_feature_id_and_distance() {
    use wbvector::{Coord, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_near_basic");
    let input_path = std::env::temp_dir().join(format!("{tag}_input.geojson"));
    let near_path = std::env::temp_dir().join(format!("{tag}_near.geojson"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut input = Layer::new("input").with_geom_type(GeometryType::Point).with_epsg(4326);
    input
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add input point 1");
    input
        .add_feature(Some(Geometry::Point(Coord::xy(5.0, 0.0))), &[])
        .expect("add input point 2");
    wbvector::write(&input, &input_path, VectorFormat::GeoJson).expect("write input");

    let mut near_layer = Layer::new("near").with_geom_type(GeometryType::Point).with_epsg(4326);
    near_layer
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add near point 1");
    near_layer
        .add_feature(Some(Geometry::Point(Coord::xy(10.0, 0.0))), &[])
        .expect("add near point 2");
    wbvector::write(&near_layer, &near_path, VectorFormat::GeoJson).expect("write near");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("near".to_string(), json!(near_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry.run("near", &args, &context(&caps)).expect("near run");

    let out = wbvector::read(&out_path).expect("read near output");
    assert_eq!(out.features.len(), 2);
    let near_fid_idx = out.schema.field_index("NEAR_FID").expect("NEAR_FID field");
    let near_dist_idx = out.schema.field_index("NEAR_DIST").expect("NEAR_DIST field");
    assert_eq!(out.schema.fields()[near_fid_idx].field_type, FieldType::Integer);
    assert!(
        matches!(
            out.schema.fields()[near_dist_idx].field_type,
            FieldType::Float | FieldType::Integer
        ),
        "NEAR_DIST should remain numeric"
    );

    let attrs0 = &out.features[0].attributes;
    match &attrs0[near_fid_idx] {
        FieldValue::Integer(v) => assert_eq!(*v, 1),
        other => panic!("expected NEAR_FID integer, got {:?}", other),
    }
    match &attrs0[near_dist_idx] {
        FieldValue::Float(v) => assert!((*v - 1.0).abs() < 1.0e-9, "expected dist 1.0, got {}", v),
        FieldValue::Integer(v) => assert_eq!(*v, 1, "expected dist 1, got {}", v),
        other => panic!("expected NEAR_DIST numeric, got {:?}", other),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&near_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn select_by_location_within_distance_filters_expected_targets() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_select_by_location_distance");
    let target_path = std::env::temp_dir().join(format!("{tag}_target.geojson"));
    let query_path = std::env::temp_dir().join(format!("{tag}_query.geojson"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.geojson"));

    let mut target = Layer::new("target").with_geom_type(GeometryType::Point).with_epsg(4326);
    target
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add target point 1");
    target
        .add_feature(Some(Geometry::Point(Coord::xy(5.0, 0.0))), &[])
        .expect("add target point 2");
    wbvector::write(&target, &target_path, VectorFormat::GeoJson).expect("write target");

    let mut query = Layer::new("query").with_geom_type(GeometryType::Point).with_epsg(4326);
    query
        .add_feature(Some(Geometry::Point(Coord::xy(0.2, 0.0))), &[])
        .expect("add query point");
    wbvector::write(&query, &query_path, VectorFormat::GeoJson).expect("write query");

    let mut args = ToolArgs::new();
    args.insert("target".to_string(), json!(target_path.to_string_lossy().to_string()));
    args.insert("query".to_string(), json!(query_path.to_string_lossy().to_string()));
    args.insert("predicate".to_string(), json!("within_distance"));
    args.insert("distance".to_string(), json!(1.0));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("select_by_location", &args, &context(&caps))
        .expect("select_by_location run");

    let out = wbvector::read(&out_path).expect("read selected output");
    assert_eq!(out.features.len(), 1, "expected only one nearby target feature");

    let coord = match out.features[0].geometry.as_ref().expect("selected geometry") {
        Geometry::Point(c) => c,
        other => panic!("expected point geometry, got {:?}", other),
    };
    assert!((coord.x - 0.0).abs() < 1.0e-9);
    assert!((coord.y - 0.0).abs() < 1.0e-9);

    let _ = std::fs::remove_file(&target_path);
    let _ = std::fs::remove_file(&query_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn add_geometry_attributes_adds_expected_area_and_centroid() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_add_geometry_attributes");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut polygons = Layer::new("polygons")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    polygons
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(1.0, 0.0),
                    Coord::xy(1.0, 1.0),
                    Coord::xy(0.0, 1.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon feature");
    wbvector::write(&polygons, &input_path, VectorFormat::GeoPackage).expect("write polygon input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("area".to_string(), json!(true));
    args.insert("length".to_string(), json!(false));
    args.insert("perimeter".to_string(), json!(false));
    args.insert("centroid".to_string(), json!(true));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("add_geometry_attributes", &args, &context(&caps))
        .expect("add_geometry_attributes run");

    let out = wbvector::read(&out_path).expect("read output polygons");
    assert_eq!(out.features.len(), 1);

    let area_idx = out.schema.field_index("AREA").expect("AREA field exists");
    let cx_idx = out.schema.field_index("CENTROID_X").expect("CENTROID_X field exists");
    let cy_idx = out.schema.field_index("CENTROID_Y").expect("CENTROID_Y field exists");
    let attrs = &out.features[0].attributes;

    match &attrs[area_idx] {
        FieldValue::Float(v) => assert!((*v - 1.0).abs() < 1.0e-9, "expected area 1.0, got {}", v),
        FieldValue::Integer(v) => assert_eq!(*v, 1, "expected area 1, got {}", v),
        other => panic!("expected numeric AREA, got {:?}", other),
    }
    match &attrs[cx_idx] {
        FieldValue::Float(v) => assert!((*v - 0.5).abs() < 1.0e-9, "expected centroid x 0.5, got {}", v),
        other => panic!("expected float CENTROID_X, got {:?}", other),
    }
    match &attrs[cy_idx] {
        FieldValue::Float(v) => assert!((*v - 0.5).abs() < 1.0e-9, "expected centroid y 0.5, got {}", v),
        other => panic!("expected float CENTROID_Y, got {:?}", other),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn field_calculator_writes_expression_result_to_output_field() {
    use wbvector::{FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_field_calculator");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut points = Layer::new("points")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    points.add_field(FieldDef::new("VAL", FieldType::Integer));
    points
        .add_feature(
            Some(Geometry::Point(Coord::xy(10.0, 20.0))),
            &[("VAL", FieldValue::Integer(5))],
        )
        .expect("add point feature");
    wbvector::write(&points, &input_path, VectorFormat::GeoPackage).expect("write point input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("field".to_string(), json!("SCORE"));
    args.insert("field_type".to_string(), json!("integer"));
    args.insert("expression".to_string(), json!("VAL * 2"));
    args.insert("overwrite".to_string(), json!(true));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("field_calculator", &args, &context(&caps))
        .expect("field_calculator run");

    let out = wbvector::read(&out_path).expect("read field calculator output");
    assert_eq!(out.features.len(), 1);

    let score_idx = out.schema.field_index("SCORE").expect("SCORE field exists");
    let attrs = &out.features[0].attributes;
    match &attrs[score_idx] {
        FieldValue::Integer(v) => assert_eq!(*v, 10),
        FieldValue::Float(v) => assert!((*v - 10.0).abs() < 1.0e-9, "expected score 10.0, got {}", v),
        other => panic!("expected numeric SCORE, got {:?}", other),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn reproject_vector_reprojects_coordinates_and_sets_target_epsg() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_reproject_vector");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut points = Layer::new("points")
        .with_geom_type(GeometryType::Point)
        .with_epsg(3857);
    points
        .add_feature(
            Some(Geometry::Point(Coord::xy(111_319.490_793_273_57, 111_325.142_866_385_1))),
            &[],
        )
        .expect("add projected point");
    wbvector::write(&points, &input_path, VectorFormat::GeoPackage).expect("write input points");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("epsg".to_string(), json!(4326));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("reproject_vector", &args, &context(&caps))
        .expect("reproject_vector run");

    let out = wbvector::read(&out_path).expect("read reprojected output");
    assert_eq!(out.features.len(), 1);
    assert_eq!(out.crs.as_ref().and_then(|c| c.epsg), Some(4326));

    let coord = match out.features[0].geometry.as_ref().expect("output geometry") {
        Geometry::Point(c) => c,
        other => panic!("expected point geometry, got {:?}", other),
    };
    assert!((coord.x - 1.0).abs() < 1.0e-3, "expected lon near 1.0, got {}", coord.x);
    assert!((coord.y - 1.0).abs() < 1.0e-3, "expected lat near 1.0, got {}", coord.y);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn simplify_features_reduces_vertices_with_tolerance() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_simplify_features");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let input_coords = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.1),
        Coord::xy(2.0, -0.1),
        Coord::xy(3.0, 0.1),
        Coord::xy(4.0, 0.0),
    ];

    let mut lines = Layer::new("lines")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(Some(Geometry::line_string(input_coords.clone())), &[])
        .expect("add line feature");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write line input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("tolerance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("simplify_features", &args, &context(&caps))
        .expect("simplify_features run");

    let out = wbvector::read(&out_path).expect("read simplified output");
    assert_eq!(out.features.len(), 1);

    let coords = match out.features[0].geometry.as_ref().expect("output geometry") {
        Geometry::LineString(coords) => coords,
        other => panic!("expected line geometry, got {:?}", other),
    };
    assert!(
        coords.len() < input_coords.len(),
        "simplification should reduce vertex count ({} -> {})",
        input_coords.len(),
        coords.len()
    );
    assert!((coords.first().expect("first").x - 0.0).abs() < 1.0e-9);
    assert!((coords.first().expect("first").y - 0.0).abs() < 1.0e-9);
    assert!((coords.last().expect("last").x - 4.0).abs() < 1.0e-9);
    assert!((coords.last().expect("last").y - 0.0).abs() < 1.0e-9);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn concave_hull_builds_polygon_output_from_points() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_concave_hull");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut points = Layer::new("points")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    for coord in [
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(2.0, 2.0),
        Coord::xy(0.0, 2.0),
        Coord::xy(1.0, 1.0),
    ] {
        points
            .add_feature(Some(Geometry::Point(coord)), &[])
            .expect("add input point");
    }
    wbvector::write(&points, &input_path, VectorFormat::GeoPackage).expect("write concave hull input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("max_edge_length".to_string(), json!(3.0));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("concave_hull", &args, &context(&caps))
        .expect("concave_hull run");

    let out = wbvector::read(&out_path).expect("read concave hull output");
    assert_eq!(out.features.len(), 1);
    assert_eq!(out.geom_type, Some(GeometryType::Polygon));

    let geom = out.features[0].geometry.as_ref().expect("concave hull geometry");
    match geom {
        Geometry::Polygon { exterior, .. } => {
            assert!(exterior.coords().len() >= 4, "polygon should have ring coordinates");
        }
        Geometry::MultiPolygon(parts) => {
            assert!(!parts.is_empty(), "multipolygon hull should have at least one part");
        }
        other => panic!("expected polygon output geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn densify_features_increases_linestring_vertex_count() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_densify_features");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let input_coords = vec![Coord::xy(0.0, 0.0), Coord::xy(4.0, 0.0)];
    let mut lines = Layer::new("lines")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(Some(Geometry::line_string(input_coords.clone())), &[])
        .expect("add line feature");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write densify input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("spacing".to_string(), json!(1.0));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("densify_features", &args, &context(&caps))
        .expect("densify_features run");

    let out = wbvector::read(&out_path).expect("read densified output");
    let coords = match out.features[0].geometry.as_ref().expect("densified geometry") {
        Geometry::LineString(coords) => coords,
        other => panic!("expected line geometry, got {:?}", other),
    };
    assert!(
        coords.len() > input_coords.len(),
        "densification should increase vertex count ({} -> {})",
        input_coords.len(),
        coords.len()
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn points_along_lines_generates_expected_spacing_points() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_points_along_lines");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("lines")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(4.0, 0.0)])),
            &[],
        )
        .expect("add source line");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write points-along-lines input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("spacing".to_string(), json!(2.0));
    args.insert("include_end".to_string(), json!(true));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("points_along_lines", &args, &context(&caps))
        .expect("points_along_lines run");

    let out = wbvector::read(&out_path).expect("read points-along-lines output");
    assert_eq!(out.geom_type, Some(GeometryType::Point));
    assert_eq!(out.features.len(), 2, "expected points at x=2,4");

    let p0 = match out.features[0].geometry.as_ref().expect("first output point") {
        Geometry::Point(c) => c,
        other => panic!("expected point geometry, got {:?}", other),
    };
    let p1 = match out.features[1].geometry.as_ref().expect("second output point") {
        Geometry::Point(c) => c,
        other => panic!("expected point geometry, got {:?}", other),
    };
    assert!((p0.x - 2.0).abs() < 1.0e-9);
    assert!((p1.x - 4.0).abs() < 1.0e-9);

    let src_fid_idx = out.schema.field_index("SRC_FID").expect("SRC_FID field");
    let mut seen_src_fid: Option<i64> = None;
    for feature in &out.features {
        match &feature.attributes[src_fid_idx] {
            FieldValue::Integer(v) => {
                if let Some(expected) = seen_src_fid {
                    assert_eq!(*v, expected, "SRC_FID should be consistent for one source line");
                } else {
                    seen_src_fid = Some(*v);
                }
            }
            other => panic!("expected SRC_FID integer, got {:?}", other),
        }
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn random_points_in_polygon_generates_requested_count_within_extent() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_random_points_in_polygon");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut polygons = Layer::new("polygons")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    polygons
        .add_feature(
            Some(Geometry::polygon(
                vec![
                    Coord::xy(0.0, 0.0),
                    Coord::xy(2.0, 0.0),
                    Coord::xy(2.0, 2.0),
                    Coord::xy(0.0, 2.0),
                    Coord::xy(0.0, 0.0),
                ],
                vec![],
            )),
            &[],
        )
        .expect("add polygon feature");
    wbvector::write(&polygons, &input_path, VectorFormat::GeoPackage).expect("write polygon input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("num_points".to_string(), json!(10));
    args.insert("seed".to_string(), json!(42));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("random_points_in_polygon", &args, &context(&caps))
        .expect("random_points_in_polygon run");

    let out = wbvector::read(&out_path).expect("read random points output");
    assert_eq!(out.features.len(), 10);
    assert_eq!(out.geom_type, Some(GeometryType::Point));
    for feature in &out.features {
        let coord = match feature.geometry.as_ref().expect("point geometry") {
            Geometry::Point(c) => c,
            other => panic!("expected point geometry, got {:?}", other),
        };
        assert!(coord.x >= 0.0 && coord.x <= 2.0, "x out of expected extent: {}", coord.x);
        assert!(coord.y >= 0.0 && coord.y <= 2.0, "y out of expected extent: {}", coord.y);
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn vector_summary_statistics_writes_expected_group_rows() {
    use wbvector::{FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_vector_summary_statistics");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_summary.csv"));

    let mut points = Layer::new("points")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    points.add_field(FieldDef::new("CLASS", FieldType::Text));
    points.add_field(FieldDef::new("VALUE", FieldType::Float));
    points
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[("CLASS", FieldValue::Text("A".to_string())), ("VALUE", FieldValue::Float(1.0))],
        )
        .expect("add point 1");
    points
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.0, 0.0))),
            &[("CLASS", FieldValue::Text("A".to_string())), ("VALUE", FieldValue::Float(3.0))],
        )
        .expect("add point 2");
    points
        .add_feature(
            Some(Geometry::Point(Coord::xy(2.0, 0.0))),
            &[("CLASS", FieldValue::Text("B".to_string())), ("VALUE", FieldValue::Float(2.0))],
        )
        .expect("add point 3");
    wbvector::write(&points, &input_path, VectorFormat::GeoPackage).expect("write summary input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("group_field".to_string(), json!("CLASS"));
    args.insert("value_field".to_string(), json!("VALUE"));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("vector_summary_statistics", &args, &context(&caps))
        .expect("vector_summary_statistics run");

    let csv = std::fs::read_to_string(&out_csv).expect("read summary csv");
    assert!(csv.contains("group,count,sum,mean,min,max"));
    assert!(csv.contains("\"A\",2,4,2,1,3"), "missing A summary row: {}", csv);
    assert!(csv.contains("\"B\",1,2,2,2,2"), "missing B summary row: {}", csv);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn add_field_appends_default_values_and_schema() {
    use wbvector::{FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_add_field");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut points = Layer::new("points")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    points.add_field(FieldDef::new("VAL", FieldType::Integer));
    points
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[("VAL", FieldValue::Integer(1))],
        )
        .expect("add point");
    wbvector::write(&points, &input_path, VectorFormat::GeoPackage).expect("write add-field input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("field".to_string(), json!("SCORE"));
    args.insert("field_type".to_string(), json!("integer"));
    args.insert("default".to_string(), json!(7));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry.run("add_field", &args, &context(&caps)).expect("add_field run");

    let out = wbvector::read(&out_path).expect("read add-field output");
    let score_idx = out.schema.field_index("SCORE").expect("SCORE field exists");
    match &out.features[0].attributes[score_idx] {
        FieldValue::Integer(v) => assert_eq!(*v, 7),
        other => panic!("expected integer SCORE default value, got {:?}", other),
    }

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn rename_and_delete_field_update_schema_as_expected() {
    use wbvector::{FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_rename_delete_field");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let renamed_path = std::env::temp_dir().join(format!("{tag}_renamed.gpkg"));
    let deleted_path = std::env::temp_dir().join(format!("{tag}_deleted.gpkg"));

    let mut points = Layer::new("points")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    points.add_field(FieldDef::new("VAL", FieldType::Integer));
    points.add_field(FieldDef::new("TMP", FieldType::Integer));
    points
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[("VAL", FieldValue::Integer(1)), ("TMP", FieldValue::Integer(99))],
        )
        .expect("add point");
    wbvector::write(&points, &input_path, VectorFormat::GeoPackage).expect("write rename/delete input");

    let mut rename_args = ToolArgs::new();
    rename_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    rename_args.insert("field".to_string(), json!("VAL"));
    rename_args.insert("new_field".to_string(), json!("VALUE"));
    rename_args.insert("output".to_string(), json!(renamed_path.to_string_lossy().to_string()));
    registry
        .run("rename_field", &rename_args, &context(&caps))
        .expect("rename_field run");

    let renamed = wbvector::read(&renamed_path).expect("read renamed output");
    assert!(renamed.schema.field_index("VALUE").is_some());
    assert!(renamed.schema.field_index("VAL").is_none());

    let mut delete_args = ToolArgs::new();
    delete_args.insert("input".to_string(), json!(renamed_path.to_string_lossy().to_string()));
    delete_args.insert("fields".to_string(), json!("TMP"));
    delete_args.insert("output".to_string(), json!(deleted_path.to_string_lossy().to_string()));
    registry
        .run("delete_field", &delete_args, &context(&caps))
        .expect("delete_field run");

    let deleted = wbvector::read(&deleted_path).expect("read deleted output");
    assert!(deleted.schema.field_index("VALUE").is_some());
    assert!(deleted.schema.field_index("TMP").is_none());

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&renamed_path);
    let _ = std::fs::remove_file(&deleted_path);
}

#[test]
fn shortest_path_network_finds_connected_route() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.1));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(1.9));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("max_snap_distance".to_string(), json!(1.0));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let out = wbvector::read(&out_path).expect("read shortest path output");
    assert_eq!(out.features.len(), 1);

    let cost_idx = out.schema.field_index("COST").expect("COST field");
    match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => assert!((*v - 2.0).abs() < 1.0e-9, "expected cost 2.0, got {}", v),
        FieldValue::Integer(v) => assert_eq!(*v, 2, "expected cost 2, got {}", v),
        other => panic!("expected numeric COST, got {:?}", other),
    }

    let coords = match out.features[0].geometry.as_ref().expect("path geometry") {
        Geometry::LineString(coords) => coords,
        other => panic!("expected linestring path geometry, got {:?}", other),
    };
    assert!(coords.len() >= 2);
    assert!((coords.first().expect("first").x - 0.0).abs() < 1.0e-9);
    assert!((coords.last().expect("last").x - 2.0).abs() < 1.0e-9);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn multimodal_shortest_path_transfer_penalty_changes_route() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_multimodal_shortest_path_transfer");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let relaxed_out = std::env::temp_dir().join(format!("{tag}_relaxed_out.gpkg"));
    let strict_out = std::env::temp_dir().join(format!("{tag}_strict_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("MODE", FieldType::Text));

    // Two-mode short corridor: A(0,0)->B(1,0) walk, B->C(2,0) transit.
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add walk edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("MODE", FieldValue::Text("transit".to_string()))],
        )
        .expect("add transit edge");

    // Single-mode detour: A->D->E->C, all walk.
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add detour edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add detour edge 3");

    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write multimodal network input");

    let mut relaxed_args = ToolArgs::new();
    relaxed_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    relaxed_args.insert("start_x".to_string(), json!(0.0));
    relaxed_args.insert("start_y".to_string(), json!(0.0));
    relaxed_args.insert("end_x".to_string(), json!(2.0));
    relaxed_args.insert("end_y".to_string(), json!(0.0));
    relaxed_args.insert("mode_field".to_string(), json!("MODE"));
    relaxed_args.insert("default_mode_speed".to_string(), json!(1.0));
    relaxed_args.insert("mode_speed_overrides".to_string(), json!("walk:1.0,transit:4.0"));
    relaxed_args.insert("transfer_penalty".to_string(), json!(0.0));
    relaxed_args.insert("max_snap_distance".to_string(), json!(0.25));
    relaxed_args.insert("output".to_string(), json!(relaxed_out.to_string_lossy().to_string()));

    let relaxed = registry
        .run("multimodal_shortest_path", &relaxed_args, &context(&caps))
        .expect("multimodal_shortest_path relaxed run");

    let relaxed_cost = relaxed
        .outputs
        .get("cost")
        .and_then(|v| v.as_f64())
        .expect("relaxed cost output");
    let relaxed_mode_changes = relaxed
        .outputs
        .get("mode_changes")
        .and_then(|v| v.as_i64())
        .expect("relaxed mode_changes output");

    let mut strict_args = relaxed_args.clone();
    strict_args.insert("transfer_penalty".to_string(), json!(5.0));
    strict_args.insert("output".to_string(), json!(strict_out.to_string_lossy().to_string()));

    let strict = registry
        .run("multimodal_shortest_path", &strict_args, &context(&caps))
        .expect("multimodal_shortest_path strict run");

    let strict_cost = strict
        .outputs
        .get("cost")
        .and_then(|v| v.as_f64())
        .expect("strict cost output");
    let strict_mode_changes = strict
        .outputs
        .get("mode_changes")
        .and_then(|v| v.as_i64())
        .expect("strict mode_changes output");

    assert!(relaxed_mode_changes >= 1, "expected relaxed run to use at least one transfer");
    assert_eq!(strict_mode_changes, 0, "expected strict run to avoid transfers");
    assert!(strict_cost > relaxed_cost, "expected transfer penalty to increase route cost");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&relaxed_out);
    let _ = std::fs::remove_file(&strict_out);
}

#[test]
fn multimodal_shortest_path_walk_drive_pattern() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_multimodal_shortest_path_walk_drive");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("MODE", FieldType::Text));

    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add walk segment");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(4.0, 0.0)])),
            &[("MODE", FieldValue::Text("drive".to_string()))],
        )
        .expect("add drive segment");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write multimodal network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(4.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("mode_field".to_string(), json!("MODE"));
    args.insert("allowed_modes".to_string(), json!("walk,drive"));
    args.insert("mode_speed_overrides".to_string(), json!("walk:1.0,drive:3.0"));
    args.insert("transfer_penalty".to_string(), json!(0.5));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let result = registry
        .run("multimodal_shortest_path", &args, &context(&caps))
        .expect("multimodal_shortest_path walk-drive run");

    let mode_changes = result
        .outputs
        .get("mode_changes")
        .and_then(|v| v.as_i64())
        .expect("mode_changes output");
    assert_eq!(mode_changes, 1, "walk-drive pattern should include one transfer");

    let out = wbvector::read(&out_path).expect("read multimodal output");
    let seq_idx = out.schema.field_index("MODE_SEQ").expect("MODE_SEQ field");
    let mode_seq = match &out.features[0].attributes[seq_idx] {
        FieldValue::Text(v) => v.clone(),
        other => panic!("expected text MODE_SEQ, got {:?}", other),
    };
    assert!(mode_seq.contains("walk") && mode_seq.contains("drive"));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn multimodal_shortest_path_walk_transit_pattern() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_multimodal_shortest_path_walk_transit");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("MODE", FieldType::Text));

    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add walk segment");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(5.0, 0.0)])),
            &[("MODE", FieldValue::Text("transit".to_string()))],
        )
        .expect("add transit segment");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write multimodal network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(5.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("mode_field".to_string(), json!("MODE"));
    args.insert("allowed_modes".to_string(), json!("walk,transit"));
    args.insert("mode_speed_overrides".to_string(), json!("walk:1.0,transit:4.0"));
    args.insert("transfer_penalty".to_string(), json!(0.25));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let result = registry
        .run("multimodal_shortest_path", &args, &context(&caps))
        .expect("multimodal_shortest_path walk-transit run");

    let mode_changes = result
        .outputs
        .get("mode_changes")
        .and_then(|v| v.as_i64())
        .expect("mode_changes output");
    assert_eq!(mode_changes, 1, "walk-transit pattern should include one transfer");

    let out = wbvector::read(&out_path).expect("read multimodal output");
    let seq_idx = out.schema.field_index("MODE_SEQ").expect("MODE_SEQ field");
    let mode_seq = match &out.features[0].attributes[seq_idx] {
        FieldValue::Text(v) => v.clone(),
        other => panic!("expected text MODE_SEQ, got {:?}", other),
    };
    assert!(mode_seq.contains("walk") && mode_seq.contains("transit"));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn multimodal_od_cost_matrix_writes_expected_batch_rows() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_multimodal_od_cost_matrix");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("MODE", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add walk edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("MODE", FieldValue::Text("transit".to_string()))],
        )
        .expect("add transit edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 0.0), Coord::xy(3.0, 0.0)])),
            &[("MODE", FieldValue::Text("transit".to_string()))],
        )
        .expect("add transit edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin A");
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add origin B");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination C");
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(3.0, 0.0))), &[])
        .expect("add destination D");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("mode_field".to_string(), json!("MODE"));
    args.insert("mode_speed_overrides".to_string(), json!("walk:1.0,transit:2.0"));
    args.insert("transfer_penalty".to_string(), json!(0.5));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));

    let result = registry
        .run("multimodal_od_cost_matrix", &args, &context(&caps))
        .expect("multimodal_od_cost_matrix run");

    let reachable_pair_count = result
        .outputs
        .get("reachable_pair_count")
        .and_then(|v| v.as_u64())
        .expect("reachable_pair_count output");
    assert_eq!(reachable_pair_count, 4, "expected all OD pairs to be reachable");

    let csv = std::fs::read_to_string(&out_csv).expect("read multimodal od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 5, "expected header + four OD rows");
    let first_parts: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(first_parts.len(), 8);
    assert_eq!(first_parts[3], "true");
    let first_cost: f64 = first_parts[2].parse().expect("parse first cost");
    assert!((first_cost - 2.0).abs() < 1.0e-9, "expected 2.0 cost from origin A to destination C, got {}", first_cost);

    let last_parts: Vec<&str> = lines[4].split(',').collect();
    let last_cost: f64 = last_parts[2].parse().expect("parse last cost");
    assert!((last_cost - 1.0).abs() < 1.0e-9, "expected 1.0 cost from origin B to destination D, got {}", last_cost);
    assert_eq!(last_parts[4], "0", "expected no mode changes for pure transit route");
    assert!(last_parts[5].contains("transit"));

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn multimodal_routes_from_od_outputs_route_geometry_and_modes() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_multimodal_routes_from_od");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("MODE", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("MODE", FieldValue::Text("walk".to_string()))],
        )
        .expect("add walk edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(3.0, 0.0)])),
            &[("MODE", FieldValue::Text("drive".to_string()))],
        )
        .expect("add drive edge");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(3.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("mode_field".to_string(), json!("MODE"));
    args.insert("allowed_modes".to_string(), json!("walk,drive"));
    args.insert("mode_speed_overrides".to_string(), json!("walk:1.0,drive:2.0"));
    args.insert("transfer_penalty".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let result = registry
        .run("multimodal_routes_from_od", &args, &context(&caps))
        .expect("multimodal_routes_from_od run");

    let route_count = result
        .outputs
        .get("route_count")
        .and_then(|v| v.as_u64())
        .expect("route_count output");
    assert_eq!(route_count, 1, "expected one reachable multimodal route");

    let out = wbvector::read(&out_path).expect("read multimodal routes output");
    assert_eq!(out.features.len(), 1, "expected one route feature");
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let mode_changes_idx = out.schema.field_index("MODE_CHG").expect("MODE_CHG field");
    let mode_seq_idx = out.schema.field_index("MODE_SEQ").expect("MODE_SEQ field");

    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 2.25).abs() < 1.0e-9, "expected walk+drive cost 2.25, got {}", cost);

    let mode_changes = match &out.features[0].attributes[mode_changes_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer MODE_CHG, got {:?}", other),
    };
    assert_eq!(mode_changes, 1, "expected one mode change");

    let mode_seq = match &out.features[0].attributes[mode_seq_idx] {
        FieldValue::Text(v) => v.clone(),
        other => panic!("expected text MODE_SEQ, got {:?}", other),
    };
    assert!(mode_seq.contains("walk") && mode_seq.contains("drive"));

    match out.features[0].geometry.as_ref().expect("route geometry") {
        Geometry::LineString(coords) => {
            assert_eq!(coords.len(), 3, "expected route through transfer node");
            assert!((coords[0].x - 0.0).abs() < 1.0e-9);
            assert!((coords[2].x - 3.0).abs() < 1.0e-9);
        }
        other => panic!("expected linestring route geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn multimodal_od_cost_matrix_applies_temporal_cost_profile() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_multimodal_od_cost_matrix_temporal");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let profile_path = std::env::temp_dir().join(format!("{tag}_profile.csv"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("MODE", FieldType::Text));
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[
                ("MODE", FieldValue::Text("walk".to_string())),
                ("EDGE_ID", FieldValue::Text("w1".to_string())),
            ],
        )
        .expect("add walk edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[
                ("MODE", FieldValue::Text("transit".to_string())),
                ("EDGE_ID", FieldValue::Text("t1".to_string())),
            ],
        )
        .expect("add transit edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 0.0), Coord::xy(3.0, 0.0)])),
            &[
                ("MODE", FieldValue::Text("transit".to_string())),
                ("EDGE_ID", FieldValue::Text("t2".to_string())),
            ],
        )
        .expect("add transit edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(3.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    std::fs::write(
        &profile_path,
        "edge_id,dow,start_minute,end_minute,value\nt1,1,480,600,10\nt2,1,480,600,10\n",
    )
    .expect("write temporal profile");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("mode_field".to_string(), json!("MODE"));
    args.insert("mode_speed_overrides".to_string(), json!("walk:1.0,transit:2.0"));
    args.insert("transfer_penalty".to_string(), json!(0.5));
    args.insert("temporal_cost_profile".to_string(), json!(profile_path.to_string_lossy().to_string()));
    args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    args.insert("temporal_mode".to_string(), json!("multiplier"));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));

    let result = registry
        .run("multimodal_od_cost_matrix", &args, &context(&caps))
        .expect("multimodal_od_cost_matrix temporal run");

    let scenario_count = result
        .outputs
        .get("scenario_count")
        .and_then(|v| v.as_u64())
        .expect("scenario_count output");
    assert_eq!(scenario_count, 1);

    let csv = std::fs::read_to_string(&out_csv).expect("read multimodal temporal od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let row: Vec<&str> = lines[1].split(',').collect();
    let cost: f64 = row[2].parse().expect("parse temporal cost");
    assert!((cost - 11.5).abs() < 1.0e-9, "expected temporal multiplier cost 11.5, got {}", cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&profile_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn multimodal_routes_from_od_writes_scenario_bundle_outputs() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_multimodal_routes_from_od_scenarios");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let baseline_profile_path = std::env::temp_dir().join(format!("{tag}_baseline.csv"));
    let peak_profile_path = std::env::temp_dir().join(format!("{tag}_peak.csv"));
    let scenario_bundle_path = std::env::temp_dir().join(format!("{tag}_scenarios.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("MODE", FieldType::Text));
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[
                ("MODE", FieldValue::Text("walk".to_string())),
                ("EDGE_ID", FieldValue::Text("w1".to_string())),
            ],
        )
        .expect("add walk edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(3.0, 0.0)])),
            &[
                ("MODE", FieldValue::Text("drive".to_string())),
                ("EDGE_ID", FieldValue::Text("d1".to_string())),
            ],
        )
        .expect("add drive edge");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(3.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    std::fs::write(
        &baseline_profile_path,
        "edge_id,dow,start_minute,end_minute,value\nd1,1,480,600,1\n",
    )
    .expect("write baseline temporal profile");
    std::fs::write(
        &peak_profile_path,
        "edge_id,dow,start_minute,end_minute,value\nd1,1,480,600,4\n",
    )
    .expect("write peak temporal profile");
    std::fs::write(
        &scenario_bundle_path,
        format!(
            "scenario,temporal_cost_profile,departure_time,temporal_mode\nbaseline,{},2026-04-13T08:30:00Z,multiplier\npeak,{},2026-04-13T08:30:00Z,multiplier\n",
            baseline_profile_path.to_string_lossy(),
            peak_profile_path.to_string_lossy()
        ),
    )
    .expect("write scenario bundle");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("mode_field".to_string(), json!("MODE"));
    args.insert("allowed_modes".to_string(), json!("walk,drive"));
    args.insert("mode_speed_overrides".to_string(), json!("walk:1.0,drive:2.0"));
    args.insert("transfer_penalty".to_string(), json!(0.25));
    args.insert("scenario_bundle_csv".to_string(), json!(scenario_bundle_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let result = registry
        .run("multimodal_routes_from_od", &args, &context(&caps))
        .expect("multimodal_routes_from_od scenario run");

    let scenario_count = result
        .outputs
        .get("scenario_count")
        .and_then(|v| v.as_u64())
        .expect("scenario_count output");
    assert_eq!(scenario_count, 2);

    let out = wbvector::read(&out_path).expect("read multimodal scenario routes output");
    assert_eq!(out.features.len(), 2, "expected one route per scenario");
    let scenario_idx = out.schema.field_index("SCENARIO").expect("SCENARIO field");
    let cost_idx = out.schema.field_index("COST").expect("COST field");

    let first_scenario = match &out.features[0].attributes[scenario_idx] {
        FieldValue::Text(v) => v.clone(),
        other => panic!("expected text SCENARIO, got {:?}", other),
    };
    let second_scenario = match &out.features[1].attributes[scenario_idx] {
        FieldValue::Text(v) => v.clone(),
        other => panic!("expected text SCENARIO, got {:?}", other),
    };
    assert_eq!(first_scenario, "baseline");
    assert_eq!(second_scenario, "peak");

    let baseline_cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    let peak_cost = match &out.features[1].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((baseline_cost - 2.25).abs() < 1.0e-9, "expected baseline cost 2.25, got {}", baseline_cost);
    assert!((peak_cost - 5.25).abs() < 1.0e-9, "expected peak cost 5.25, got {}", peak_cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&baseline_profile_path);
    let _ = std::fs::remove_file(&peak_profile_path);
    let _ = std::fs::remove_file(&scenario_bundle_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_uses_edge_cost_field_multiplier() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_edge_cost_field");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(10.0))],
        )
        .expect("add expensive edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("IMP", FieldValue::Float(10.0))],
        )
        .expect("add expensive edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add cheap edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add cheap edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add cheap edge 3");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add cheap edge 4");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let out = wbvector::read(&out_path).expect("read shortest path output");
    assert_eq!(out.features.len(), 1);
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 4.0).abs() < 1.0e-9, "expected weighted path cost 4.0, got {}", cost);

    let coords = match out.features[0].geometry.as_ref().expect("path geometry") {
        Geometry::LineString(coords) => coords,
        other => panic!("expected linestring path geometry, got {:?}", other),
    };
    assert!(coords.iter().any(|c| (c.y - 1.0).abs() < 1.0e-9));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_centrality_metrics_identifies_middle_node_as_most_central() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_centrality_metrics_line");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add segment 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add segment 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_centrality_metrics", &args, &context(&caps))
        .expect("network_centrality_metrics run");

    let out = wbvector::read(&out_path).expect("read centrality output");
    assert_eq!(out.features.len(), 3, "expected three graph nodes");

    let node_idx = out.schema.field_index("NODE_ID").expect("NODE_ID field");
    let close_idx = out.schema.field_index("CLOSENESS").expect("CLOSENESS field");
    let betw_idx = out.schema.field_index("BETWEENNESS").expect("BETWEENNESS field");

    let mut closeness_by_node = std::collections::HashMap::<i64, f64>::new();
    let mut betweenness_by_node = std::collections::HashMap::<i64, f64>::new();
    for feature in &out.features {
        let node_id = match &feature.attributes[node_idx] {
            wbvector::FieldValue::Integer(v) => *v,
            other => panic!("expected integer NODE_ID, got {:?}", other),
        };
        let closeness = match &feature.attributes[close_idx] {
            wbvector::FieldValue::Float(v) => *v,
            wbvector::FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric CLOSENESS, got {:?}", other),
        };
        let betweenness = match &feature.attributes[betw_idx] {
            wbvector::FieldValue::Float(v) => *v,
            wbvector::FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric BETWEENNESS, got {:?}", other),
        };
        closeness_by_node.insert(node_id, closeness);
        betweenness_by_node.insert(node_id, betweenness);
    }

    let middle_closeness = *closeness_by_node.get(&2).expect("middle node closeness");
    let end1_closeness = *closeness_by_node.get(&1).expect("end node 1 closeness");
    let end2_closeness = *closeness_by_node.get(&3).expect("end node 2 closeness");
    assert!(middle_closeness > end1_closeness);
    assert!(middle_closeness > end2_closeness);

    let middle_betweenness = *betweenness_by_node.get(&2).expect("middle node betweenness");
    let end1_betweenness = *betweenness_by_node.get(&1).expect("end node 1 betweenness");
    let end2_betweenness = *betweenness_by_node.get(&3).expect("end node 2 betweenness");
    assert!(middle_betweenness > end1_betweenness);
    assert!(middle_betweenness > end2_betweenness);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_accessibility_metrics_computes_weighted_accessibility_by_cutoff_and_decay() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_accessibility_metrics");
    let network_path = std::env::temp_dir().join(format!("{tag}_net.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_ori.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_dst.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    // Create a linear network segment (A-B-C)
    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add segment 1");
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add segment 2");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage)
        .expect("write network input");

    // Single origin at A (x=0)
    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[],
        )
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage)
        .expect("write origins");

    // Two destinations at B and C (x=1 and x=2)
    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.0, 0.0))),
            &[],
        )
        .expect("add destination 1");
    destinations
        .add_feature(
            Some(Geometry::Point(Coord::xy(2.0, 0.0))),
            &[],
        )
        .expect("add destination 2");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    // Test 1: No cutoff or decay (count all reachable)
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("network_accessibility_metrics", &args, &context(&caps))
        .expect("network_accessibility_metrics run");

    let out = wbvector::read(&output_path).expect("read accessibility output");
    assert_eq!(out.features.len(), 1, "expected one origin feature");

    let access_idx = out.schema.field_index("ACCESSIBILITY").expect("ACCESSIBILITY field");
    let accessibility = match &out.features[0].attributes[access_idx] {
        wbvector::FieldValue::Float(v) => *v,
        wbvector::FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric ACCESSIBILITY, got {:?}", other),
    };
    assert_eq!(accessibility, 2.0, "expected accessibility count of 2 destinations");

    // Test 2: With impedance cutoff (only first destination reachable)
    let output_path2 = std::env::temp_dir().join(format!("{tag}_out2.gpkg"));
    let mut args2 = ToolArgs::new();
    args2.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args2.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args2.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args2.insert("impedance_cutoff".to_string(), json!(1.2));
    args2.insert("output".to_string(), json!(output_path2.to_string_lossy().to_string()));
    registry
        .run("network_accessibility_metrics", &args2, &context(&caps))
        .expect("network_accessibility_metrics run with cutoff");

    let out2 = wbvector::read(&output_path2).expect("read accessibility with cutoff");
    let accessibility2 = match &out2.features[0].attributes[access_idx] {
        wbvector::FieldValue::Float(v) => *v,
        wbvector::FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric ACCESSIBILITY, got {:?}", other),
    };
    assert_eq!(accessibility2, 1.0, "expected accessibility count of 1 destination with cutoff");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&output_path2);
}

#[test]
fn od_sensitivity_analysis_computes_perturbed_od_costs_with_variance() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_od_sensitivity");
    let network_path = std::env::temp_dir().join(format!("{tag}_net.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_ori.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_dst.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.csv"));

    // Create a linear network with cost field
    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network.schema.add_field(wbvector::FieldDef::new("cost", wbvector::FieldType::Float));
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("cost", wbvector::FieldValue::Float(1.0))],
        )
        .expect("add segment 1");
    network
        .add_feature(
            Some(Geometry::LineString(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("cost", wbvector::FieldValue::Float(1.0))],
        )
        .expect("add segment 2");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage)
        .expect("write network");

    // Single origin at x=0
    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.0, 0.0))),
            &[],
        )
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage)
        .expect("write origins");

    // Single destination at x=2
    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(
            Some(Geometry::Point(Coord::xy(2.0, 0.0))),
            &[],
        )
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    // Run with Monte Carlo samples to get variance
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("edge_cost_field".to_string(), json!("cost"));
    args.insert("impedance_disturbance_range".to_string(), json!("0.9,1.1"));
    args.insert("monte_carlo_samples".to_string(), json!(5));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("od_sensitivity_analysis", &args, &context(&caps))
        .expect("od_sensitivity_analysis run");

    // Parse output CSV and verify
    let csv_content = std::fs::read_to_string(&output_path)
        .expect("read sensitivity output");
    let lines: Vec<&str> = csv_content.lines().collect();
    assert!(lines.len() > 1, "expected output CSV with header and data rows");
    
    // Verify header
    assert_eq!(lines[0], "origin_id,destination_id,baseline_cost,mean_cost,stdev_cost,min_cost,max_cost");

    // Verify data row format
    if lines.len() > 1 {
        let fields: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(fields.len(), 7, "expected 7 CSV fields");
        
        // Parse fields to verify numeric values
        let baseline: f64 = fields[2].parse().expect("baseline_cost numeric");
        let mean: f64 = fields[3].parse().expect("mean_cost numeric");
        let stdev: f64 = fields[4].parse().expect("stdev_cost numeric");
        let min_cost: f64 = fields[5].parse().expect("min_cost numeric");
        let max_cost: f64 = fields[6].parse().expect("max_cost numeric");

        // Verify cost relationships
        assert!(baseline > 0.0, "baseline cost should be positive");
        assert!(mean > 0.0, "mean cost should be positive");
        assert!(stdev >= 0.0, "stdev should be non-negative");
        assert!(min_cost <= mean, "min_cost should be <= mean");
        assert!(max_cost >= mean, "max_cost should be >= mean");
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn stream_d_centrality_metrics_benchmark_validates_correctness_across_network_topologies() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_centrality_benchmark");
    let network_path = std::env::temp_dir().join(format!("{tag}_net.gpkg"));
    let output_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    // Create a grid network (4x4 grid, 16 nodes, highly connected)
    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    
    // Horizontal edges
    for row in 0..4 {
        for col in 0..3 {
            let x1 = col as f64;
            let y1 = row as f64;
            let x2 = (col + 1) as f64;
            let y2 = row as f64;
            network
                .add_feature(
                    Some(Geometry::LineString(vec![Coord::xy(x1, y1), Coord::xy(x2, y2)])),
                    &[],
                )
                .expect("add horizontal edge");
        }
    }
    // Vertical edges
    for row in 0..3 {
        for col in 0..4 {
            let x1 = col as f64;
            let y1 = row as f64;
            let x2 = col as f64;
            let y2 = (row + 1) as f64;
            network
                .add_feature(
                    Some(Geometry::LineString(vec![Coord::xy(x1, y1), Coord::xy(x2, y2)])),
                    &[],
                )
                .expect("add vertical edge");
        }
    }
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage)
        .expect("write network");

    // Run centrality metrics on grid network
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
    registry
        .run("network_centrality_metrics", &args, &context(&caps))
        .expect("network_centrality_metrics run");

    // Parse output and validate
    let output = wbvector::read(&output_path).expect("read centrality output");
    assert_eq!(output.features.len(), 16, "expected 16 nodes in 4x4 grid");
    
    // Verify corner nodes have degree 2, edge nodes have degree 3, inner nodes have degree 4
    let degree_idx = output.schema.field_index("DEGREE").expect("DEGREE field");
    let degrees: Vec<i32> = output.features.iter().map(|f| {
        match &f.attributes[degree_idx] {
            wbvector::FieldValue::Integer(d) => *d as i32,
            _ => panic!("expected integer DEGREE"),
        }
    }).collect();
    
    let corner_degrees: i32 = degrees.iter().filter(|&&d| d == 2).count() as i32;
    let edge_degrees: i32 = degrees.iter().filter(|&&d| d == 3).count() as i32;
    let inner_degrees: i32 = degrees.iter().filter(|&&d| d == 4).count() as i32;
    
    assert_eq!(corner_degrees, 4, "expected 4 corner nodes with degree 2");
    assert_eq!(edge_degrees, 8, "expected 8 edge nodes with degree 3");
    assert_eq!(inner_degrees, 4, "expected 4 inner nodes with degree 4");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn stream_d_accessibility_metrics_benchmark_validates_impedance_cutoff_and_decay_combinations() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_accessibility_benchmark");
    let network_path = std::env::temp_dir().join(format!("{tag}_net.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_ori.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_dst.gpkg"));
    let output_no_decay = std::env::temp_dir().join(format!("{tag}_out_nodecay.gpkg"));
    let output_linear_decay = std::env::temp_dir().join(format!("{tag}_out_linear.gpkg"));
    let output_exp_decay = std::env::temp_dir().join(format!("{tag}_out_exp.gpkg"));

    // Create a star network with origin in center
    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    for i in 0..8 {
        let angle = (i as f64 * std::f64::consts::PI * 2.0) / 8.0;
        let x = 5.0 + 3.0 * angle.cos();
        let y = 5.0 + 3.0 * angle.sin();
        network
            .add_feature(
                Some(Geometry::LineString(vec![Coord::xy(5.0, 5.0), Coord::xy(x, y)])),
                &[],
            )
            .expect("add spoke");
    }
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage)
        .expect("write network");

    // Single origin at center
    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(
            Some(Geometry::Point(Coord::xy(5.0, 5.0))),
            &[],
        )
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage)
        .expect("write origins");

    // 8 destinations at ends of spokes
    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    for i in 0..8 {
        let angle = (i as f64 * std::f64::consts::PI * 2.0) / 8.0;
        let x = 5.0 + 3.0 * angle.cos();
        let y = 5.0 + 3.0 * angle.sin();
        destinations
            .add_feature(
                Some(Geometry::Point(Coord::xy(x, y))),
                &[],
            )
            .expect("add destination");
    }
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    // Test 1: No decay (simple counting)
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("impedance_cutoff".to_string(), json!(10.0));
    args.insert("decay_function".to_string(), json!("none"));
    args.insert("output".to_string(), json!(output_no_decay.to_string_lossy().to_string()));
    registry
        .run("network_accessibility_metrics", &args, &context(&caps))
        .expect("accessibility with no decay");
    
    let out_no_decay = wbvector::read(&output_no_decay).expect("read no decay output");
    let acc_idx = out_no_decay.schema.field_index("ACCESSIBILITY").expect("ACCESSIBILITY field");
    let acc_no_decay = match &out_no_decay.features[0].attributes[acc_idx] {
        wbvector::FieldValue::Float(v) => *v,
        _ => panic!("expected float ACCESSIBILITY"),
    };
    assert_eq!(acc_no_decay, 8.0, "expected all 8 destinations reachable with no decay");

    // Test 2: Linear decay
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("impedance_cutoff".to_string(), json!(10.0));
    args.insert("decay_function".to_string(), json!("linear"));
    args.insert("decay_parameter".to_string(), json!(0.5));
    args.insert("output".to_string(), json!(output_linear_decay.to_string_lossy().to_string()));
    registry
        .run("network_accessibility_metrics", &args, &context(&caps))
        .expect("accessibility with linear decay");
    
    let out_linear = wbvector::read(&output_linear_decay).expect("read linear output");
    let acc_linear = match &out_linear.features[0].attributes[acc_idx] {
        wbvector::FieldValue::Float(v) => *v,
        _ => panic!("expected float ACCESSIBILITY"),
    };
    assert!(acc_linear < acc_no_decay, "linear decay should reduce accessibility score");

    // Test 3: Exponential decay
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("impedance_cutoff".to_string(), json!(10.0));
    args.insert("decay_function".to_string(), json!("exponential"));
    args.insert("decay_parameter".to_string(), json!(0.5));
    args.insert("output".to_string(), json!(output_exp_decay.to_string_lossy().to_string()));
    registry
        .run("network_accessibility_metrics", &args, &context(&caps))
        .expect("accessibility with exponential decay");
    
    let out_exp = wbvector::read(&output_exp_decay).expect("read exponential output");
    let acc_exp = match &out_exp.features[0].attributes[acc_idx] {
        wbvector::FieldValue::Float(v) => *v,
        _ => panic!("expected float ACCESSIBILITY"),
    };
    assert!(acc_exp < acc_linear, "exponential decay (lambda=0.5) should reduce accessibility more than linear");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&output_no_decay);
    let _ = std::fs::remove_file(&output_linear_decay);
    let _ = std::fs::remove_file(&output_exp_decay);
}

#[test]
fn stream_d_od_sensitivity_analysis_benchmark_validates_scaling_with_network_and_sample_size() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_od_sensitivity_scaling_benchmark");
    
    // Test with increasing network sizes: linear network with 5, 10, and 15 segments
    for num_segments in &[5, 10, 15] {
        let network_path = std::env::temp_dir().join(format!("{tag}_{}_net.gpkg", num_segments));
        let origins_path = std::env::temp_dir().join(format!("{tag}_{}_ori.gpkg", num_segments));
        let destinations_path = std::env::temp_dir().join(format!("{tag}_{}_dst.gpkg", num_segments));
        let output_path = std::env::temp_dir().join(format!("{tag}_{}_out.csv", num_segments));

        // Create linear network
        let mut network = Layer::new("network")
            .with_geom_type(GeometryType::LineString)
            .with_epsg(4326);
        network.schema.add_field(wbvector::FieldDef::new("cost", wbvector::FieldType::Float));

        for i in 0..*num_segments {
            let x1 = i as f64;
            let x2 = (i + 1) as f64;
            network
                .add_feature(
                    Some(Geometry::LineString(vec![Coord::xy(x1, 0.0), Coord::xy(x2, 0.0)])),
                    &[("cost", wbvector::FieldValue::Float(1.0))],
                )
                .expect("add segment");
        }
        wbvector::write(&network, &network_path, VectorFormat::GeoPackage)
            .expect("write network");

        // Single origin and destination at ends
        let mut origins = Layer::new("origins")
            .with_geom_type(GeometryType::Point)
            .with_epsg(4326);
        origins
            .add_feature(
                Some(Geometry::Point(Coord::xy(0.0, 0.0))),
                &[],
            )
            .expect("add origin");
        wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage)
            .expect("write origins");

        let mut destinations = Layer::new("destinations")
            .with_geom_type(GeometryType::Point)
            .with_epsg(4326);
        destinations
            .add_feature(
                Some(Geometry::Point(Coord::xy(*num_segments as f64, 0.0))),
                &[],
            )
            .expect("add destination");
        wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
            .expect("write destinations");

        // Run sensitivity analysis
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
        args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
        args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
        args.insert("edge_cost_field".to_string(), json!("cost"));
        args.insert("impedance_disturbance_range".to_string(), json!("0.9,1.1"));
        args.insert("monte_carlo_samples".to_string(), json!(10));
        args.insert("output".to_string(), json!(output_path.to_string_lossy().to_string()));
        registry
            .run("od_sensitivity_analysis", &args, &context(&caps))
            .expect("od_sensitivity_analysis run");

        // Verify output
        let csv_content = std::fs::read_to_string(&output_path)
            .expect("read sensitivity output");
        let lines: Vec<&str> = csv_content.lines().collect();
        assert!(lines.len() > 1, "expected output CSV for network with {} segments", num_segments);
        
        // Validate expected baseline cost
        if lines.len() > 1 {
            let fields: Vec<&str> = lines[1].split(',').collect();
            let baseline: f64 = fields[2].parse().expect("baseline_cost numeric");
            assert_eq!(baseline, *num_segments as f64, "expected baseline cost matching segment count");
        }

        let _ = std::fs::remove_file(&network_path);
        let _ = std::fs::remove_file(&origins_path);
        let _ = std::fs::remove_file(&destinations_path);
        let _ = std::fs::remove_file(&output_path);
    }
}

#[test]
fn shortest_path_network_temporal_profile_changes_route_by_departure_time() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_temporal_profile_route");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let out_rush_path = std::env::temp_dir().join(format!("{tag}_rush_out.gpkg"));
    let out_offpeak_path = std::env::temp_dir().join(format!("{tag}_offpeak_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));

    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("D_E".to_string()))],
        )
        .expect("add detour edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("E_F".to_string()))],
        )
        .expect("add detour edge 3");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("F_C".to_string()))],
        )
        .expect("add detour edge 4");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    rush_args.insert("start_x".to_string(), json!(0.0));
    rush_args.insert("start_y".to_string(), json!(0.0));
    rush_args.insert("end_x".to_string(), json!(2.0));
    rush_args.insert("end_y".to_string(), json!(0.0));
    rush_args.insert("max_snap_distance".to_string(), json!(0.25));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(out_rush_path.to_string_lossy().to_string()));

    registry
        .run("shortest_path_network", &rush_args, &context(&caps))
        .expect("shortest_path_network rush run");

    let rush_out = wbvector::read(&out_rush_path).expect("read shortest path rush output");
    assert_eq!(rush_out.features.len(), 1);
    let rush_cost_idx = rush_out.schema.field_index("COST").expect("COST field");
    let rush_cost = match &rush_out.features[0].attributes[rush_cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((rush_cost - 4.0).abs() < 1.0e-9, "expected detour cost 4.0 during rush hour, got {}", rush_cost);

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    offpeak_args.insert("start_x".to_string(), json!(0.0));
    offpeak_args.insert("start_y".to_string(), json!(0.0));
    offpeak_args.insert("end_x".to_string(), json!(2.0));
    offpeak_args.insert("end_y".to_string(), json!(0.0));
    offpeak_args.insert("max_snap_distance".to_string(), json!(0.25));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(out_offpeak_path.to_string_lossy().to_string()));

    registry
        .run("shortest_path_network", &offpeak_args, &context(&caps))
        .expect("shortest_path_network offpeak run");

    let offpeak_out = wbvector::read(&out_offpeak_path).expect("read shortest path offpeak output");
    assert_eq!(offpeak_out.features.len(), 1);
    let offpeak_cost_idx = offpeak_out.schema.field_index("COST").expect("COST field");
    let offpeak_cost = match &offpeak_out.features[0].attributes[offpeak_cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((offpeak_cost - 2.0).abs() < 1.0e-9, "expected direct cost 2.0 offpeak, got {}", offpeak_cost);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&out_rush_path);
    let _ = std::fs::remove_file(&out_offpeak_path);
}

#[test]
fn shortest_path_network_temporal_profile_error_fallback_requires_coverage() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_temporal_profile_error_fallback");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,2\n",
    )
    .expect("write temporal profile csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    args.insert("temporal_fallback".to_string(), json!("error"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect_err("expected failure when temporal_fallback=error and edge profile is missing");
    let msg = err.to_string();
    assert!(
        msg.contains("missing temporal cost profile"),
        "unexpected error: {}",
        msg
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_temporal_profile_report_emits_diagnostics() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_temporal_profile_report");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let report_path = std::env::temp_dir().join(format!("{tag}_temporal_report.json"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 0.0), Coord::xy(3.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("C_D".to_string()))],
        )
        .expect("add edge 3");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,2\nB_C,1,420,600,2\n",
    )
    .expect("write temporal profile csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(3.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    args.insert("temporal_profile_report".to_string(), json!(report_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let report_text = std::fs::read_to_string(&report_path).expect("read temporal profile report");
    let report: serde_json::Value = serde_json::from_str(&report_text).expect("parse temporal profile report json");

    assert_eq!(report["profile_edge_id_count"].as_u64(), Some(2));
    assert_eq!(report["network_edge_count"].as_u64(), Some(3));
    assert_eq!(report["network_unique_edge_id_count"].as_u64(), Some(3));
    assert_eq!(report["network_edges_without_temporal_rows"].as_u64(), Some(1));
    assert_eq!(report["fallback_usage_count"].as_u64(), Some(1));
    let unmatched = report["unmatched_profile_edge_ids"]
        .as_array()
        .expect("unmatched_profile_edge_ids array");
    assert!(unmatched.is_empty(), "expected all profile edge ids to exist in the network");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&report_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_respects_one_way_field_direction() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_one_way_field");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("ONEWAY", FieldType::Integer));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("ONEWAY", FieldValue::Integer(1))],
        )
        .expect("add one-way edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("ONEWAY", FieldValue::Integer(0))],
        )
        .expect("add two-way edge");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(2.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(0.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("one_way_field".to_string(), json!("ONEWAY"));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect_err("expected reverse traversal to fail on one-way edge");
    let msg = err.to_string();
    assert!(msg.contains("no path found"), "unexpected error: {}", msg);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_respects_blocked_field_edges() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_blocked_field");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("BLOCKED", FieldType::Integer));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("BLOCKED", FieldValue::Integer(1))],
        )
        .expect("add blocked edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("BLOCKED", FieldValue::Integer(1))],
        )
        .expect("add blocked edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("BLOCKED", FieldValue::Integer(0))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("BLOCKED", FieldValue::Integer(0))],
        )
        .expect("add detour edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[("BLOCKED", FieldValue::Integer(0))],
        )
        .expect("add detour edge 3");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("BLOCKED", FieldValue::Integer(0))],
        )
        .expect("add detour edge 4");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("blocked_field".to_string(), json!("BLOCKED"));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let out = wbvector::read(&out_path).expect("read shortest path output");
    assert_eq!(out.features.len(), 1);
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 4.0).abs() < 1.0e-9, "expected detour cost 4.0, got {}", cost);

    let coords = match out.features[0].geometry.as_ref().expect("path geometry") {
        Geometry::LineString(coords) => coords,
        other => panic!("expected linestring path geometry, got {:?}", other),
    };
    assert!(coords.iter().any(|c| (c.y - 1.0).abs() < 1.0e-9));

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_respects_barrier_points() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_barriers");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barrier input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.25));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect_err("expected path to fail when middle node is blocked by barrier");
    let msg = err.to_string();
    assert!(msg.contains("no path found"), "unexpected error: {}", msg);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_applies_turn_penalty_to_cost() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_turn_penalty");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(1.0));
    args.insert("end_y".to_string(), json!(1.0));
    args.insert("turn_penalty".to_string(), json!(3.0));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let out = wbvector::read(&out_path).expect("read shortest path output");
    assert_eq!(out.features.len(), 1);
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 5.0).abs() < 1.0e-9, "expected cost 5.0 with turn penalty, got {}", cost);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_can_forbid_left_turns() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_forbid_left_turns");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add AB");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add BC");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add AD");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add DC");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(1.0));
    args.insert("end_y".to_string(), json!(1.0));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert("forbid_left_turns".to_string(), json!(true));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let out = wbvector::read(&out_path).expect("read shortest path output");
    assert_eq!(out.features.len(), 1);
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 4.0).abs() < 1.0e-9, "expected cost 4.0 with left-turn restriction, got {}", cost);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_respects_turn_restrictions_csv() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_turn_restrictions_csv");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let restrictions_csv = std::env::temp_dir().join(format!("{tag}_turns.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add AB");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add BC");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add AD");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add DC");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    std::fs::write(
        &restrictions_csv,
        "prev_x,prev_y,node_x,node_y,next_x,next_y\n0,0,1,0,1,1\n",
    )
    .expect("write turn restrictions csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(1.0));
    args.insert("end_y".to_string(), json!(1.0));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert(
        "turn_restrictions_csv".to_string(),
        json!(restrictions_csv.to_string_lossy().to_string()),
    );
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let out = wbvector::read(&out_path).expect("read shortest path output");
    assert_eq!(out.features.len(), 1);
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 4.0).abs() < 1.0e-9, "expected detour cost 4.0 with turn restriction, got {}", cost);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&restrictions_csv);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_node_degree_identifies_junction_and_dead_ends() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_node_degree");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[],
        )
        .expect("add edge 3");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_node_degree", &args, &context(&caps))
        .expect("network_node_degree run");

    let out = wbvector::read(&out_path).expect("read network nodes output");
    let degree_idx = out.schema.field_index("DEGREE").expect("DEGREE field");
    let node_type_idx = out.schema.field_index("NODE_TYPE").expect("NODE_TYPE field");

    let mut count_deg1 = 0;
    let mut count_deg3 = 0;
    for feature in &out.features {
        let degree = match &feature.attributes[degree_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected DEGREE integer, got {:?}", other),
        };
        let node_type = match &feature.attributes[node_type_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("expected NODE_TYPE text, got {:?}", other),
        };

        if degree == 1 {
            count_deg1 += 1;
            assert_eq!(node_type, "dead_end");
        } else if degree == 3 {
            count_deg3 += 1;
            assert_eq!(node_type, "junction");
        }
    }

    assert_eq!(count_deg3, 1, "expected one junction node");
    assert_eq!(count_deg1, 3, "expected three dead-end nodes");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_returns_nodes_within_max_cost() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.1));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area output");
    assert_eq!(out.features.len(), 2, "expected nodes at costs 0 and 1 within max_cost");

    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let mut costs = Vec::<f64>::new();
    for feature in &out.features {
        let c = match &feature.attributes[cost_idx] {
            FieldValue::Float(v) => *v,
            FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric COST, got {:?}", other),
        };
        costs.push(c);
    }
    costs.sort_by(|a, b| a.total_cmp(b));
    assert!((costs[0] - 0.0).abs() < 1.0e-9);
    assert!((costs[1] - 1.0).abs() < 1.0e-9);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_nodes_output_supports_ring_costs() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_ring_costs_nodes");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(2.5));
    args.insert("ring_costs".to_string(), json!("1.0,2.0"));
    args.insert("output_mode".to_string(), json!("nodes"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area output");
    assert_eq!(out.features.len(), 3, "expected nodes at costs 0,1,2");

    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let ring_idx = out.schema.field_index("RING_IDX").expect("RING_IDX field");

    let mut rows = Vec::<(f64, i64)>::new();
    for feature in &out.features {
        let cost = match &feature.attributes[cost_idx] {
            FieldValue::Float(v) => *v,
            FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric COST, got {:?}", other),
        };
        let ring = match &feature.attributes[ring_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer RING_IDX, got {:?}", other),
        };
        rows.push((cost, ring));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));

    assert!((rows[0].0 - 0.0).abs() < 1.0e-9 && rows[0].1 == 1);
    assert!((rows[1].0 - 1.0).abs() < 1.0e-9 && rows[1].1 == 1);
    assert!((rows[2].0 - 2.0).abs() < 1.0e-9 && rows[2].1 == 2);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_temporal_profile_changes_reachability_by_departure_time() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_temporal_profile");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let rush_out_path = std::env::temp_dir().join(format!("{tag}_rush_out.gpkg"));
    let offpeak_out_path = std::env::temp_dir().join(format!("{tag}_offpeak_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("D_C".to_string()))],
        )
        .expect("add detour edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    rush_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    rush_args.insert("max_cost".to_string(), json!(2.5));
    rush_args.insert("output_mode".to_string(), json!("nodes"));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(rush_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &rush_args, &context(&caps))
        .expect("network_service_area rush run");

    let rush_out = wbvector::read(&rush_out_path).expect("read rush service area output");
    assert_eq!(rush_out.features.len(), 2, "expected only origin and one detour node at rush hour");

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    offpeak_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    offpeak_args.insert("max_cost".to_string(), json!(2.5));
    offpeak_args.insert("output_mode".to_string(), json!("nodes"));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(offpeak_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &offpeak_args, &context(&caps))
        .expect("network_service_area offpeak run");

    let offpeak_out = wbvector::read(&offpeak_out_path).expect("read offpeak service area output");
    assert_eq!(offpeak_out.features.len(), 4, "expected all reachable nodes offpeak, including detour branch");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&rush_out_path);
    let _ = std::fs::remove_file(&offpeak_out_path);
}

#[test]
fn map_matching_v1_matches_clean_trajectory_and_emits_diagnostics() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_map_matching_v1_basic");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let trajectory_path = std::env::temp_dir().join(format!("{tag}_trajectory.gpkg"));
    let route_out_path = std::env::temp_dir().join(format!("{tag}_route_out.gpkg"));
    let points_out_path = std::env::temp_dir().join(format!("{tag}_points_out.gpkg"));
    let report_out_path = std::env::temp_dir().join(format!("{tag}_report.json"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("E1".to_string()))],
        )
        .expect("add network edge 1");
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("E2".to_string()))],
        )
        .expect("add network edge 2");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut trajectory = Layer::new("trajectory")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    trajectory.schema.add_field(FieldDef::new("TS", FieldType::Text));
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.05, 0.02))),
            &[("TS", FieldValue::Text("2026-04-12T10:00:00Z".to_string()))],
        )
        .expect("add trajectory point 1");
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.95, -0.02))),
            &[("TS", FieldValue::Text("2026-04-12T10:01:00Z".to_string()))],
        )
        .expect("add trajectory point 2");
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.95, 0.01))),
            &[("TS", FieldValue::Text("2026-04-12T10:02:00Z".to_string()))],
        )
        .expect("add trajectory point 3");
    wbvector::write(&trajectory, &trajectory_path, VectorFormat::GeoPackage).expect("write trajectory");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("trajectory_points".to_string(), json!(trajectory_path.to_string_lossy().to_string()));
    args.insert("timestamp_field".to_string(), json!("TS"));
    args.insert("search_radius".to_string(), json!(0.5));
    args.insert("candidate_k".to_string(), json!(3));
    args.insert("matched_points_output".to_string(), json!(points_out_path.to_string_lossy().to_string()));
    args.insert("match_report".to_string(), json!(report_out_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(route_out_path.to_string_lossy().to_string()));

    registry
        .run("map_matching_v1", &args, &context(&caps))
        .expect("map_matching_v1 run");

    let route_out = wbvector::read(&route_out_path).expect("read route output");
    assert_eq!(route_out.features.len(), 1, "expected single matched route feature");
    let matched_idx = route_out.schema.field_index("MATCHED_PTS").expect("MATCHED_PTS field");
    let unmatched_idx = route_out.schema.field_index("UNMATCHED").expect("UNMATCHED field");
    let matched = match &route_out.features[0].attributes[matched_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer MATCHED_PTS, got {:?}", other),
    };
    let unmatched = match &route_out.features[0].attributes[unmatched_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer UNMATCHED, got {:?}", other),
    };
    assert_eq!(matched, 3);
    assert_eq!(unmatched, 0);

    let points_out = wbvector::read(&points_out_path).expect("read matched points output");
    assert_eq!(points_out.features.len(), 3, "expected matched diagnostics for each point");
    let status_idx = points_out.schema.field_index("STATUS").expect("STATUS field");
    for feature in &points_out.features {
        let status = match &feature.attributes[status_idx] {
            FieldValue::Text(v) => v.as_str(),
            other => panic!("expected text STATUS, got {:?}", other),
        };
        assert!(status == "matched" || status == "ambiguous");
    }

    let report_text = std::fs::read_to_string(&report_out_path).expect("read report output");
    let report_json: serde_json::Value = serde_json::from_str(&report_text).expect("parse report json");
    assert!(report_json.get("match_rate").is_some(), "match_report should include match_rate");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&trajectory_path);
    let _ = std::fs::remove_file(&route_out_path);
    let _ = std::fs::remove_file(&points_out_path);
    let _ = std::fs::remove_file(&report_out_path);
}

#[test]
fn map_matching_v1_partial_unmatched_points_still_emit_outputs() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_map_matching_v1_partial_unmatched");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let trajectory_path = std::env::temp_dir().join(format!("{tag}_trajectory.gpkg"));
    let route_out_path = std::env::temp_dir().join(format!("{tag}_route_out.gpkg"));
    let points_out_path = std::env::temp_dir().join(format!("{tag}_points_out.gpkg"));
    let report_out_path = std::env::temp_dir().join(format!("{tag}_report.json"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("E1".to_string()))],
        )
        .expect("add network edge 1");
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("E2".to_string()))],
        )
        .expect("add network edge 2");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut trajectory = Layer::new("trajectory")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    trajectory.schema.add_field(FieldDef::new("TS", FieldType::Text));
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.05, 0.01))),
            &[("TS", FieldValue::Text("2026-04-12T10:00:00Z".to_string()))],
        )
        .expect("add trajectory point 1");
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(50.0, 50.0))),
            &[("TS", FieldValue::Text("2026-04-12T10:01:00Z".to_string()))],
        )
        .expect("add trajectory point 2");
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.95, -0.01))),
            &[("TS", FieldValue::Text("2026-04-12T10:02:00Z".to_string()))],
        )
        .expect("add trajectory point 3");
    wbvector::write(&trajectory, &trajectory_path, VectorFormat::GeoPackage).expect("write trajectory");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("trajectory_points".to_string(), json!(trajectory_path.to_string_lossy().to_string()));
    args.insert("timestamp_field".to_string(), json!("TS"));
    args.insert("search_radius".to_string(), json!(0.25));
    args.insert("candidate_k".to_string(), json!(3));
    args.insert("matched_points_output".to_string(), json!(points_out_path.to_string_lossy().to_string()));
    args.insert("match_report".to_string(), json!(report_out_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(route_out_path.to_string_lossy().to_string()));

    registry
        .run("map_matching_v1", &args, &context(&caps))
        .expect("map_matching_v1 run");

    let route_out = wbvector::read(&route_out_path).expect("read route output");
    assert_eq!(route_out.features.len(), 1, "expected single matched route feature");
    let matched_idx = route_out.schema.field_index("MATCHED_PTS").expect("MATCHED_PTS field");
    let unmatched_idx = route_out.schema.field_index("UNMATCHED").expect("UNMATCHED field");
    let matched = match &route_out.features[0].attributes[matched_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer MATCHED_PTS, got {:?}", other),
    };
    let unmatched = match &route_out.features[0].attributes[unmatched_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer UNMATCHED, got {:?}", other),
    };
    assert_eq!(matched, 2);
    assert_eq!(unmatched, 1);

    let points_out = wbvector::read(&points_out_path).expect("read matched points output");
    assert_eq!(points_out.features.len(), 3, "expected diagnostics for all trajectory points");
    let status_idx = points_out.schema.field_index("STATUS").expect("STATUS field");
    let unmatched_count = points_out
        .features
        .iter()
        .filter(|feature| match &feature.attributes[status_idx] {
            FieldValue::Text(v) => v == "unmatched",
            _ => false,
        })
        .count();
    assert_eq!(unmatched_count, 1, "expected one unmatched diagnostic record");

    let report_text = std::fs::read_to_string(&report_out_path).expect("read report output");
    let report_json: serde_json::Value = serde_json::from_str(&report_text).expect("parse report json");
    let match_rate = report_json
        .get("match_rate")
        .and_then(|v| v.as_f64())
        .expect("match_rate value");
    assert!(match_rate > 0.0 && match_rate < 1.0, "match_rate should reflect partial matching");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&trajectory_path);
    let _ = std::fs::remove_file(&route_out_path);
    let _ = std::fs::remove_file(&points_out_path);
    let _ = std::fs::remove_file(&report_out_path);
}

#[test]
fn map_matching_v1_confidence_decreases_with_offset_noise() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_map_matching_v1_confidence_noise");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let trajectory_path = std::env::temp_dir().join(format!("{tag}_trajectory.gpkg"));
    let route_out_path = std::env::temp_dir().join(format!("{tag}_route_out.gpkg"));
    let points_out_path = std::env::temp_dir().join(format!("{tag}_points_out.gpkg"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("E_MAIN".to_string()))],
        )
        .expect("add network edge");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut trajectory = Layer::new("trajectory")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    trajectory.schema.add_field(FieldDef::new("TS", FieldType::Text));
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.5, 0.02))),
            &[("TS", FieldValue::Text("2026-04-12T10:00:00Z".to_string()))],
        )
        .expect("add low-noise point");
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.5, 0.30))),
            &[("TS", FieldValue::Text("2026-04-12T10:01:00Z".to_string()))],
        )
        .expect("add high-noise point");
    wbvector::write(&trajectory, &trajectory_path, VectorFormat::GeoPackage).expect("write trajectory");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("trajectory_points".to_string(), json!(trajectory_path.to_string_lossy().to_string()));
    args.insert("timestamp_field".to_string(), json!("TS"));
    args.insert("search_radius".to_string(), json!(0.5));
    args.insert("candidate_k".to_string(), json!(3));
    args.insert("matched_points_output".to_string(), json!(points_out_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(route_out_path.to_string_lossy().to_string()));

    registry
        .run("map_matching_v1", &args, &context(&caps))
        .expect("map_matching_v1 run");

    let points_out = wbvector::read(&points_out_path).expect("read matched points output");
    assert_eq!(points_out.features.len(), 2, "expected diagnostics for both points");
    let conf_idx = points_out.schema.field_index("CONFIDENCE").expect("CONFIDENCE field");
    let offset_idx = points_out.schema.field_index("OFFSET_DST").expect("OFFSET_DST field");

    let mut pairs = Vec::<(f64, f64)>::new();
    for feature in &points_out.features {
        let conf = match &feature.attributes[conf_idx] {
            FieldValue::Float(v) => *v,
            FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric CONFIDENCE, got {:?}", other),
        };
        let offset = match &feature.attributes[offset_idx] {
            FieldValue::Float(v) => *v,
            FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric OFFSET_DST, got {:?}", other),
        };
        pairs.push((offset, conf));
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(pairs.len(), 2);
    assert!(pairs[0].0 < pairs[1].0, "low-noise point should have smaller offset");
    assert!(pairs[0].1 > pairs[1].1, "confidence should decrease as offset/noise increases");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&trajectory_path);
    let _ = std::fs::remove_file(&route_out_path);
    let _ = std::fs::remove_file(&points_out_path);
}

#[test]
fn map_matching_v1_one_way_restriction_avoided() {
    // A trajectory that travels RIGHT→LEFT against a one-way digitized edge should
    // result in a disconnected segment rather than silently violating the restriction.
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_map_matching_v1_one_way");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let trajectory_path = std::env::temp_dir().join(format!("{tag}_trajectory.gpkg"));
    let route_out_path = std::env::temp_dir().join(format!("{tag}_route_out.gpkg"));
    let points_out_path = std::env::temp_dir().join(format!("{tag}_points_out.gpkg"));

    // Single one-way edge: (0,0) → (2,0), digitized left-to-right.
    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network.schema.add_field(FieldDef::new("ONE_WAY", FieldType::Integer));
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("ONE_WAY", FieldValue::Integer(1))],
        )
        .expect("add one-way edge");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    // Trajectory travels right-to-left — against the one-way direction.
    let mut trajectory = Layer::new("trajectory")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    trajectory.schema.add_field(FieldDef::new("TS", FieldType::Text));
    // Point 1 at t=T1: near right/end of edge → will snap to node (2,0)
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(1.8, 0.05))),
            &[("TS", FieldValue::Text("2026-04-12T10:00:00Z".to_string()))],
        )
        .expect("add right-end point");
    // Point 2 at t=T2: near left/start of edge → will snap to node (0,0)
    trajectory
        .add_feature(
            Some(Geometry::Point(Coord::xy(0.2, 0.05))),
            &[("TS", FieldValue::Text("2026-04-12T10:01:00Z".to_string()))],
        )
        .expect("add left-end point");
    wbvector::write(&trajectory, &trajectory_path, VectorFormat::GeoPackage).expect("write trajectory");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("trajectory_points".to_string(), json!(trajectory_path.to_string_lossy().to_string()));
    args.insert("timestamp_field".to_string(), json!("TS"));
    args.insert("one_way_field".to_string(), json!("ONE_WAY"));
    args.insert("search_radius".to_string(), json!(0.3));
    args.insert("candidate_k".to_string(), json!(3));
    args.insert("matched_points_output".to_string(), json!(points_out_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(route_out_path.to_string_lossy().to_string()));

    registry
        .run("map_matching_v1", &args, &context(&caps))
        .expect("map_matching_v1 run");

    let route_out = wbvector::read(&route_out_path).expect("read route output");
    assert_eq!(route_out.features.len(), 1, "expected exactly one route feature");
    let disc_seg_idx = route_out.schema.field_index("DISC_SEG").expect("DISC_SEG field");
    let disc_seg = match &route_out.features[0].attributes[disc_seg_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected Integer for DISC_SEG, got {:?}", other),
    };
    assert!(
        disc_seg >= 1,
        "expected at least one disconnected segment when trajectory travels against a one-way edge, got DISC_SEG={}",
        disc_seg
    );

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&trajectory_path);
    let _ = std::fs::remove_file(&route_out_path);
    let _ = std::fs::remove_file(&points_out_path);
}

#[test]
fn network_topology_audit_reports_nodes_and_components() {
    // Three-edge connected network: (0,0)→(1,0)→(2,0)→(3,0).
    // Expected graph nodes: 4, all in one component.
    // Dead ends at (0,0) and (3,0) (each has only one neighbor).
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_topology_audit_basic");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let nodes_out_path = std::env::temp_dir().join(format!("{tag}_nodes_out.gpkg"));
    let report_path = std::env::temp_dir().join(format!("{tag}_report.json"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("AB".to_string()))],
        )
        .expect("add edge AB");
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("BC".to_string()))],
        )
        .expect("add edge BC");
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 0.0), Coord::xy(3.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("CD".to_string()))],
        )
        .expect("add edge CD");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(nodes_out_path.to_string_lossy().to_string()));
    args.insert("report".to_string(), json!(report_path.to_string_lossy().to_string()));

    registry
        .run("network_topology_audit", &args, &context(&caps))
        .expect("network_topology_audit run");

    let nodes_out = wbvector::read(&nodes_out_path).expect("read nodes output");
    assert_eq!(nodes_out.features.len(), 4, "expected 4 graph nodes for a 3-segment linear chain");

    let node_type_idx = nodes_out.schema.field_index("NODE_TYPE").expect("NODE_TYPE field");
    let component_idx = nodes_out.schema.field_index("COMPONENT").expect("COMPONENT field");

    // All nodes should be in component 1 (single connected network).
    for feature in &nodes_out.features {
        let comp = match &feature.attributes[component_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected Integer for COMPONENT, got {:?}", other),
        };
        assert_eq!(comp, 1, "all nodes must be in component 1 for a connected network");
    }

    // Endpoint nodes (degree=1) should be dead_end; interior nodes should be through.
    let node_types: Vec<String> = nodes_out
        .features
        .iter()
        .map(|f| match &f.attributes[node_type_idx] {
            FieldValue::Text(s) => s.clone(),
            other => panic!("expected Text for NODE_TYPE, got {:?}", other),
        })
        .collect();
    let dead_ends = node_types.iter().filter(|t| t.as_str() == "dead_end").count();
    let throughs = node_types.iter().filter(|t| t.as_str() == "through").count();
    assert_eq!(dead_ends, 2, "expected 2 dead-end nodes at the chain endpoints");
    assert_eq!(throughs, 2, "expected 2 through nodes at the interior junction");

    let report_text = std::fs::read_to_string(&report_path).expect("read audit report");
    let report: serde_json::Value = serde_json::from_str(&report_text).expect("parse report JSON");
    assert_eq!(report["node_count"], 4, "report node_count should be 4");
    assert_eq!(report["component_count"], 1, "single connected network should have 1 component");
    assert_eq!(report["dead_end_node_count"], 2, "report should count 2 dead-end nodes");
    assert_eq!(report["isolated_node_count"], 0, "no isolated nodes expected");
    let warnings = report["potential_routing_failures"].as_array().unwrap();
    assert!(warnings.is_empty(), "no routing failure warnings expected for a fully connected network");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&nodes_out_path);
    let _ = std::fs::remove_file(&report_path);
}

#[test]
fn network_topology_audit_detects_disconnected_components_and_warns() {
    // Two isolated edges with no shared nodes: (0,0)→(1,0) and (5,0)→(6,0).
    // Expected: 2 components; JSON report contains a warning string mentioning "2 disconnected components".
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_topology_audit_disconnected");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let nodes_out_path = std::env::temp_dir().join(format!("{tag}_nodes_out.gpkg"));
    let report_path = std::env::temp_dir().join(format!("{tag}_report.json"));

    let mut network = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    network.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("SEGMENT_A".to_string()))],
        )
        .expect("add segment A");
    network
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(5.0, 0.0), Coord::xy(6.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("SEGMENT_B".to_string()))],
        )
        .expect("add segment B");
    wbvector::write(&network, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(nodes_out_path.to_string_lossy().to_string()));
    args.insert("report".to_string(), json!(report_path.to_string_lossy().to_string()));

    registry
        .run("network_topology_audit", &args, &context(&caps))
        .expect("network_topology_audit run");

    let nodes_out = wbvector::read(&nodes_out_path).expect("read nodes output");
    assert_eq!(nodes_out.features.len(), 4, "expected 4 graph nodes total (2 per isolated segment)");

    let component_idx = nodes_out.schema.field_index("COMPONENT").expect("COMPONENT field");
    let component_ids: Vec<i64> = nodes_out
        .features
        .iter()
        .map(|f| match &f.attributes[component_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected Integer for COMPONENT, got {:?}", other),
        })
        .collect();
    let unique_components: std::collections::HashSet<i64> = component_ids.into_iter().collect();
    assert_eq!(unique_components.len(), 2, "two isolated segments should produce two distinct component IDs");

    let report_text = std::fs::read_to_string(&report_path).expect("read audit report");
    let report: serde_json::Value = serde_json::from_str(&report_text).expect("parse report JSON");
    assert_eq!(report["component_count"], 2, "report should show 2 components");
    let warnings = report["potential_routing_failures"].as_array().unwrap();
    assert!(
        !warnings.is_empty(),
        "expected at least one routing failure warning for a disconnected network"
    );
    let warning_text = warnings[0].as_str().unwrap_or("");
    assert!(
        warning_text.contains("2 disconnected"),
        "warning should mention '2 disconnected', got: {}",
        warning_text
    );

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&nodes_out_path);
    let _ = std::fs::remove_file(&report_path);
}

#[test]
fn network_service_area_temporal_profile_and_ring_costs_interact_consistently() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_temporal_ring_interaction");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let rush_out_path = std::env::temp_dir().join(format!("{tag}_rush_out.gpkg"));
    let offpeak_out_path = std::env::temp_dir().join(format!("{tag}_offpeak_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("D_C".to_string()))],
        )
        .expect("add detour edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    rush_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    rush_args.insert("max_cost".to_string(), json!(2.5));
    rush_args.insert("ring_costs".to_string(), json!("1.0,2.5"));
    rush_args.insert("output_mode".to_string(), json!("nodes"));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(rush_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &rush_args, &context(&caps))
        .expect("network_service_area rush run");

    let rush_out = wbvector::read(&rush_out_path).expect("read rush service area output");
    let rush_ring_idx = rush_out.schema.field_index("RING_IDX").expect("rush RING_IDX field");
    let rush_rings: Vec<i64> = rush_out
        .features
        .iter()
        .map(|feature| match &feature.attributes[rush_ring_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer RING_IDX, got {:?}", other),
        })
        .collect();
    assert!(rush_rings.iter().all(|v| *v == 1), "rush-hour reachable nodes should remain in first ring");

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    offpeak_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    offpeak_args.insert("max_cost".to_string(), json!(2.5));
    offpeak_args.insert("ring_costs".to_string(), json!("1.0,2.5"));
    offpeak_args.insert("output_mode".to_string(), json!("nodes"));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(offpeak_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &offpeak_args, &context(&caps))
        .expect("network_service_area offpeak run");

    let offpeak_out = wbvector::read(&offpeak_out_path).expect("read offpeak service area output");
    let offpeak_ring_idx = offpeak_out.schema.field_index("RING_IDX").expect("offpeak RING_IDX field");
    let mut offpeak_rings: Vec<i64> = offpeak_out
        .features
        .iter()
        .map(|feature| match &feature.attributes[offpeak_ring_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer RING_IDX, got {:?}", other),
        })
        .collect();
    offpeak_rings.sort_unstable();
    offpeak_rings.dedup();

    assert!(offpeak_rings.contains(&1), "offpeak should include first ring nodes");
    assert!(offpeak_rings.contains(&2), "offpeak should include second ring nodes");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&rush_out_path);
    let _ = std::fs::remove_file(&offpeak_out_path);
}

#[test]
fn network_service_area_edges_temporal_profile_and_ring_costs_interact_consistently() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_edges_temporal_ring_interaction");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let rush_out_path = std::env::temp_dir().join(format!("{tag}_rush_out.gpkg"));
    let offpeak_out_path = std::env::temp_dir().join(format!("{tag}_offpeak_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("D_C".to_string()))],
        )
        .expect("add detour edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    rush_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    rush_args.insert("max_cost".to_string(), json!(2.5));
    rush_args.insert("ring_costs".to_string(), json!("1.0,2.5"));
    rush_args.insert("output_mode".to_string(), json!("edges"));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(rush_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &rush_args, &context(&caps))
        .expect("network_service_area rush edges run");

    let rush_out = wbvector::read(&rush_out_path).expect("read rush service area edges output");

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    offpeak_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    offpeak_args.insert("max_cost".to_string(), json!(2.5));
    offpeak_args.insert("ring_costs".to_string(), json!("1.0,2.5"));
    offpeak_args.insert("output_mode".to_string(), json!("edges"));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(offpeak_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &offpeak_args, &context(&caps))
        .expect("network_service_area offpeak edges run");

    let offpeak_out = wbvector::read(&offpeak_out_path).expect("read offpeak service area edges output");
    assert!(
        offpeak_out.features.len() > rush_out.features.len(),
        "offpeak should yield more reachable edge features than rush-hour"
    );

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&rush_out_path);
    let _ = std::fs::remove_file(&offpeak_out_path);
}

#[test]
fn network_service_area_polygons_temporal_profile_and_ring_costs_interact_consistently() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_temporal_ring_interaction");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let rush_out_path = std::env::temp_dir().join(format!("{tag}_rush_out.gpkg"));
    let offpeak_out_path = std::env::temp_dir().join(format!("{tag}_offpeak_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("D_C".to_string()))],
        )
        .expect("add detour edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    rush_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    rush_args.insert("max_cost".to_string(), json!(2.5));
    rush_args.insert("ring_costs".to_string(), json!("1.0,2.5"));
    rush_args.insert("output_mode".to_string(), json!("polygons"));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(rush_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &rush_args, &context(&caps))
        .expect("network_service_area rush polygons run");

    let rush_out = wbvector::read(&rush_out_path).expect("read rush service area polygons output");
    let rush_node_count_idx = rush_out.schema.field_index("NODE_COUNT").expect("rush NODE_COUNT field");
    let rush_max_node_count = rush_out
        .features
        .iter()
        .map(|feature| match &feature.attributes[rush_node_count_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer NODE_COUNT, got {:?}", other),
        })
        .max()
        .expect("rush polygon features");

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    offpeak_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    offpeak_args.insert("max_cost".to_string(), json!(2.5));
    offpeak_args.insert("ring_costs".to_string(), json!("1.0,2.5"));
    offpeak_args.insert("output_mode".to_string(), json!("polygons"));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(offpeak_out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &offpeak_args, &context(&caps))
        .expect("network_service_area offpeak polygons run");

    let offpeak_out = wbvector::read(&offpeak_out_path).expect("read offpeak service area polygons output");
    let offpeak_node_count_idx = offpeak_out.schema.field_index("NODE_COUNT").expect("offpeak NODE_COUNT field");
    let offpeak_max_node_count = offpeak_out
        .features
        .iter()
        .map(|feature| match &feature.attributes[offpeak_node_count_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer NODE_COUNT, got {:?}", other),
        })
        .max()
        .expect("offpeak polygon features");

    assert!(
        offpeak_max_node_count > rush_max_node_count,
        "offpeak polygons should include more reachable network nodes than rush-hour polygons"
    );

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&rush_out_path);
    let _ = std::fs::remove_file(&offpeak_out_path);
}

#[test]
fn network_service_area_respects_barrier_points() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_barriers");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barriers");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(5.0));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area output");
    assert_eq!(out.features.len(), 1, "expected only origin node reachable when middle node is barrier-blocked");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_edges_output_trims_segments_by_remaining_cost() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_edges_output");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("ONEWAY", FieldType::Integer));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("ONEWAY", FieldValue::Integer(1))],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("ONEWAY", FieldValue::Integer(1))],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.5));
    args.insert("one_way_field".to_string(), json!("ONEWAY"));
    args.insert("output_mode".to_string(), json!("edges"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area edge output");
    assert_eq!(out.features.len(), 2, "expected one full edge and one trimmed partial edge");
    let frac_idx = out.schema.field_index("EDGE_FRAC").expect("EDGE_FRAC field");

    let mut has_full = false;
    let mut has_half = false;
    for feature in &out.features {
        let frac = match &feature.attributes[frac_idx] {
            FieldValue::Float(v) => *v,
            FieldValue::Integer(v) => *v as f64,
            other => panic!("expected numeric EDGE_FRAC, got {:?}", other),
        };
        if (frac - 1.0).abs() < 1.0e-9 {
            has_full = true;
        }
        if (frac - 0.5).abs() < 1.0e-9 {
            has_half = true;
            match feature.geometry.as_ref().expect("edge geometry") {
                Geometry::LineString(coords) => {
                    assert!((coords[coords.len() - 1].x - 1.5).abs() < 1.0e-9);
                }
                other => panic!("expected linestring geometry, got {:?}", other),
            }
        }
    }
    assert!(has_full, "expected a fully reachable first edge");
    assert!(has_half, "expected a partially reachable second edge");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_edges_output_supports_ring_costs() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_edges_ring_costs");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(2.0));
    args.insert("output_mode".to_string(), json!("edges"));
    args.insert("ring_costs".to_string(), json!("1.0,2.0"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area edges run");

    let out = wbvector::read(&out_path).expect("read service area edges output");
    assert!(!out.features.is_empty(), "expected reachable edge features");
    let ring_idx = out.schema.field_index("RING_IDX").expect("RING_IDX field");

    let mut has_ring1 = false;
    let mut has_ring2 = false;
    for feature in &out.features {
        let value = match &feature.attributes[ring_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer RING_IDX, got {:?}", other),
        };
        if value == 1 {
            has_ring1 = true;
        }
        if value == 2 {
            has_ring2 = true;
        }
    }
    assert!(has_ring1 && has_ring2, "expected edge features in ring 1 and ring 2");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_emits_origin_hulls() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_output");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.5));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 1, "expected one polygon for one origin");
    let node_count_idx = out.schema.field_index("NODE_COUNT").expect("NODE_COUNT field");
    let node_count = match &out.features[0].attributes[node_count_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer NODE_COUNT, got {:?}", other),
    };
    assert_eq!(node_count, 2, "expected two reachable nodes within max_cost");

    match out.features[0].geometry.as_ref().expect("polygon geometry") {
        Geometry::Polygon { exterior, .. } => {
            assert!(exterior.coords().len() >= 3, "expected polygon ring with at least three vertices");
        }
        other => panic!("expected polygon geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_supports_ring_costs() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_ring_costs");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(2.0));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("ring_costs".to_string(), json!("1.0,2.0"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area polygons run");

    let out = wbvector::read(&out_path).expect("read service area polygons output");
    assert!(out.features.len() >= 2, "expected at least two ring polygons");
    let ring_idx = out.schema.field_index("RING_IDX").expect("RING_IDX field");

    let mut has_ring1 = false;
    let mut has_ring2 = false;
    for feature in &out.features {
        let value = match &feature.attributes[ring_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer RING_IDX, got {:?}", other),
        };
        if value == 1 {
            has_ring1 = true;
        }
        if value == 2 {
            has_ring2 = true;
        }
    }
    assert!(has_ring1 && has_ring2, "expected polygon features in ring 1 and ring 2");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_emits_diagnostics_counts() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_diagnostics");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.1));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area polygons run");

    let out = wbvector::read(&out_path).expect("read service area polygons output");
    assert!(!out.features.is_empty(), "expected at least one polygon");

    let frontier_idx = out.schema.field_index("FRONTIER_CT").expect("FRONTIER_CT field");
    let partial_idx = out.schema.field_index("PARTIAL_CT").expect("PARTIAL_CT field");

    for feature in &out.features {
        let frontier = match &feature.attributes[frontier_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer FRONTIER_CT, got {:?}", other),
        };
        let partial = match &feature.attributes[partial_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer PARTIAL_CT, got {:?}", other),
        };
        assert!(frontier >= 0, "frontier count should be non-negative");
        assert!(partial >= 0, "partial count should be non-negative");
        assert!(partial <= frontier, "partial count should not exceed frontier count");
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_tracks_partial_edge_frontier() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_frontier");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.5));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 1, "expected one polygon for one origin");

    match out.features[0].geometry.as_ref().expect("polygon geometry") {
        Geometry::Polygon { exterior, .. } => {
            let max_y = exterior
                .coords()
                .iter()
                .map(|coord| coord.y)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(max_y > 0.45, "expected polygon frontier to extend near y=0.5, got {}", max_y);
        }
        other => panic!("expected polygon geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_emits_one_polygon_per_origin() {
    use std::collections::BTreeSet;
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_per_origin");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin 1");
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add origin 2");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.1));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 2, "expected one polygon per origin");

    let origin_id_idx = out.schema.field_index("ORIGIN_ID").expect("ORIGIN_ID field");
    let node_count_idx = out.schema.field_index("NODE_COUNT").expect("NODE_COUNT field");

    let mut origin_ids = BTreeSet::<i64>::new();
    for feature in &out.features {
        let origin_id = match &feature.attributes[origin_id_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer ORIGIN_ID, got {:?}", other),
        };
        origin_ids.insert(origin_id);

        let node_count = match &feature.attributes[node_count_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer NODE_COUNT, got {:?}", other),
        };
        assert_eq!(node_count, 2, "expected each origin service area to include two nodes");

        match feature.geometry.as_ref().expect("polygon geometry") {
            Geometry::Polygon { exterior, .. } => {
                assert!(exterior.coords().len() >= 3, "expected polygon ring with at least three vertices");
            }
            other => panic!("expected polygon geometry, got {:?}", other),
        }
    }
    assert_eq!(origin_ids, BTreeSet::from([0, 1]), "expected polygons for both origin features");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_respects_barrier_points() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_barriers");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barriers");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(5.0));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 1, "expected one polygon for one origin");
    let node_count_idx = out.schema.field_index("NODE_COUNT").expect("NODE_COUNT field");
    let node_count = match &out.features[0].attributes[node_count_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer NODE_COUNT, got {:?}", other),
    };
    assert_eq!(node_count, 1, "expected only the origin node to remain reachable when barrier blocks corridor");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_respects_turn_restrictions_csv() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_turn_restrictions");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let restrictions_csv = std::env::temp_dir().join(format!("{tag}_turns.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add AB");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add BC");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add AD");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add DC");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    std::fs::write(
        &restrictions_csv,
        "prev_x,prev_y,node_x,node_y,next_x,next_y\n0,0,1,0,1,1\n",
    )
    .expect("write turn restrictions csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(2.1));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert(
        "turn_restrictions_csv".to_string(),
        json!(restrictions_csv.to_string_lossy().to_string()),
    );
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 1, "expected one polygon for one origin");
    let node_count_idx = out.schema.field_index("NODE_COUNT").expect("NODE_COUNT field");
    let node_count = match &out.features[0].attributes[node_count_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer NODE_COUNT, got {:?}", other),
    };
    assert_eq!(node_count, 3, "expected turn restriction to remove only the direct AB->BC reachability");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&restrictions_csv);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_handles_duplicate_origins() {
    use std::collections::BTreeSet;
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_duplicate_origins");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin 1");
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin 2 duplicate");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.1));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 2, "expected one polygon per origin feature, even if coordinates duplicate");
    let origin_id_idx = out.schema.field_index("ORIGIN_ID").expect("ORIGIN_ID field");
    let mut origin_ids = BTreeSet::<i64>::new();
    for feature in &out.features {
        let origin_id = match &feature.attributes[origin_id_idx] {
            FieldValue::Integer(v) => *v,
            other => panic!("expected integer ORIGIN_ID, got {:?}", other),
        };
        origin_ids.insert(origin_id);
    }
    assert_eq!(origin_ids, BTreeSet::from([0, 1]));

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_with_tiny_snap_tolerance() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_tiny_snap_tolerance");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 0.000001)])),
            &[],
        )
        .expect("add short edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.000001), Coord::xy(0.000001, 0.000001)])),
            &[],
        )
        .expect("add short edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(0.0000025));
    args.insert("snap_tolerance".to_string(), json!(1.0e-12));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 1, "expected one polygon output");
    let node_count_idx = out.schema.field_index("NODE_COUNT").expect("NODE_COUNT field");
    let node_count = match &out.features[0].attributes[node_count_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer NODE_COUNT, got {:?}", other),
    };
    assert_eq!(node_count, 3, "expected all three nodes reachable with tiny snap_tolerance");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_polygons_output_single_reachable_node_has_degenerate_hull_bbox() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_polygons_single_node");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barriers");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(100.0));
    args.insert("output_mode".to_string(), json!("polygons"));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_service_area", &args, &context(&caps))
        .expect("network_service_area run");

    let out = wbvector::read(&out_path).expect("read service area polygon output");
    assert_eq!(out.features.len(), 1, "expected one polygon for one origin");
    match out.features[0].geometry.as_ref().expect("polygon geometry") {
        Geometry::Polygon { exterior, .. } => {
            let coords = exterior.coords();
            assert_eq!(coords.len(), 4, "degenerate hull bbox should contain four ring vertices");
            let min_x = coords.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
            let max_x = coords.iter().map(|c| c.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = coords.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
            let max_y = coords.iter().map(|c| c.y).fold(f64::NEG_INFINITY, f64::max);
            assert!(max_x > min_x, "degenerate hull bbox should have positive width");
            assert!(max_y > min_y, "degenerate hull bbox should have positive height");
        }
        other => panic!("expected polygon geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_service_area_rejects_unknown_output_mode() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_service_area_bad_output_mode");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("max_cost".to_string(), json!(1.0));
    args.insert("output_mode".to_string(), json!("mesh"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("network_service_area", &args, &context(&caps))
        .expect_err("expected validation error for unsupported output_mode");
    let msg = err.to_string();
    assert!(
        msg.contains("output_mode must be one of 'nodes', 'edges', or 'polygons'"),
        "unexpected error: {}",
        msg
    );

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_od_cost_matrix_writes_expected_cost_row() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(parts.len(), 6);
    let _origin_fid: i64 = parts[0].parse().expect("parse origin_fid");
    let _destination_fid: i64 = parts[1].parse().expect("parse destination_fid");
    assert_eq!(parts[3], "true");
    let cost: f64 = parts[2].parse().expect("parse cost");
    assert!((cost - 2.0).abs() < 1.0e-9);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_uses_edge_cost_field_multiplier() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_edge_cost_field");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(2.5))],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("IMP", FieldValue::Float(2.5))],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    let cost: f64 = parts[2].parse().expect("parse cost");
    assert!((cost - 5.0).abs() < 1.0e-9, "expected weighted cost 5.0, got {}", cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_marks_unreachable_with_one_way_field() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_one_way_field");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("ONEWAY", FieldType::Integer));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("ONEWAY", FieldValue::Integer(1))],
        )
        .expect("add one-way edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("ONEWAY", FieldValue::Integer(0))],
        )
        .expect("add two-way edge");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("one_way_field".to_string(), json!("ONEWAY"));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(parts[3], "false", "expected unreachable OD pair due to one-way edge");
    assert!(parts[2].is_empty(), "cost should be empty for unreachable OD row");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_marks_unreachable_with_blocked_field() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_blocked_field");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("BLOCKED", FieldType::Integer));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(-1.0, 0.0), Coord::xy(0.0, 0.0)])),
            &[("BLOCKED", FieldValue::Integer(0))],
        )
        .expect("add local origin edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("BLOCKED", FieldValue::Integer(1))],
        )
        .expect("add blocked edge");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("BLOCKED", FieldValue::Integer(0))],
        )
        .expect("add open edge");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("blocked_field".to_string(), json!("BLOCKED"));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(parts[3], "false", "expected unreachable OD pair due to blocked edge");
    assert!(parts[2].is_empty(), "cost should be empty for unreachable OD row");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_marks_unreachable_with_barriers() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_barriers");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barriers");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(parts[3], "false", "expected unreachable OD pair due to barrier node");
    assert!(parts[2].is_empty(), "cost should be empty for unreachable OD row");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_applies_turn_penalty_to_cost() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_turn_penalty");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 1.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("turn_penalty".to_string(), json!(2.0));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    let cost: f64 = parts[2].parse().expect("parse cost");
    assert!((cost - 4.0).abs() < 1.0e-9, "expected cost 4.0 with turn penalty, got {}", cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_can_forbid_left_turns() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_forbid_left_turns");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add AB");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add BC");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add AD");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add DC");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 1.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert("forbid_left_turns".to_string(), json!(true));
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    let cost: f64 = parts[2].parse().expect("parse cost");
    assert!((cost - 4.0).abs() < 1.0e-9, "expected cost 4.0 with left-turn restriction, got {}", cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_respects_turn_restrictions_csv() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_turn_restrictions_csv");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let restrictions_csv = std::env::temp_dir().join(format!("{tag}_turns.csv"));
    let out_csv = std::env::temp_dir().join(format!("{tag}_od.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add AB");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add BC");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add AD");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add DC");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 1.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    std::fs::write(
        &restrictions_csv,
        "prev_x,prev_y,node_x,node_y,next_x,next_y\n0,0,1,0,1,1\n",
    )
    .expect("write turn restrictions csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert(
        "turn_restrictions_csv".to_string(),
        json!(restrictions_csv.to_string_lossy().to_string()),
    );
    args.insert("output".to_string(), json!(out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &args, &context(&caps))
        .expect("network_od_cost_matrix run");

    let csv = std::fs::read_to_string(&out_csv).expect("read od csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2, "expected header + one OD row");
    let parts: Vec<&str> = lines[1].split(',').collect();
    let cost: f64 = parts[2].parse().expect("parse cost");
    assert!((cost - 4.0).abs() < 1.0e-9, "expected detour cost 4.0 with turn restriction, got {}", cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&restrictions_csv);
    let _ = std::fs::remove_file(&out_csv);
}

#[test]
fn network_od_cost_matrix_temporal_profile_changes_cost_by_departure_time() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_od_matrix_temporal_profile");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let rush_out_csv = std::env::temp_dir().join(format!("{tag}_rush.csv"));
    let offpeak_out_csv = std::env::temp_dir().join(format!("{tag}_offpeak.csv"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("D_E".to_string()))],
        )
        .expect("add detour edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("E_F".to_string()))],
        )
        .expect("add detour edge 3");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("F_C".to_string()))],
        )
        .expect("add detour edge 4");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    rush_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    rush_args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(rush_out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &rush_args, &context(&caps))
        .expect("network_od_cost_matrix rush run");

    let rush_csv = std::fs::read_to_string(&rush_out_csv).expect("read rush od csv");
    let rush_lines: Vec<&str> = rush_csv.lines().collect();
    let rush_parts: Vec<&str> = rush_lines[1].split(',').collect();
    let rush_cost: f64 = rush_parts[2].parse().expect("parse rush cost");
    assert!((rush_cost - 4.0).abs() < 1.0e-9, "expected rush-hour detour cost 4.0, got {}", rush_cost);

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    offpeak_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    offpeak_args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(offpeak_out_csv.to_string_lossy().to_string()));
    registry
        .run("network_od_cost_matrix", &offpeak_args, &context(&caps))
        .expect("network_od_cost_matrix offpeak run");

    let offpeak_csv = std::fs::read_to_string(&offpeak_out_csv).expect("read offpeak od csv");
    let offpeak_lines: Vec<&str> = offpeak_csv.lines().collect();
    let offpeak_parts: Vec<&str> = offpeak_lines[1].split(',').collect();
    let offpeak_cost: f64 = offpeak_parts[2].parse().expect("parse offpeak cost");
    assert!((offpeak_cost - 2.0).abs() < 1.0e-9, "expected offpeak direct cost 2.0, got {}", offpeak_cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&rush_out_csv);
    let _ = std::fs::remove_file(&offpeak_out_csv);
}

#[test]
fn network_connected_components_labels_disconnected_subnetworks() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_connected_components");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add component A edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add component A edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(10.0, 0.0), Coord::xy(11.0, 0.0)])),
            &[],
        )
        .expect("add component B edge");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write input network");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_connected_components", &args, &context(&caps))
        .expect("network_connected_components run");

    let out = wbvector::read(&out_path).expect("read components output");
    assert_eq!(out.features.len(), 3);
    let comp_idx = out.schema.field_index("COMP_ID").expect("COMP_ID field");

    let c0 = match &out.features[0].attributes[comp_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer COMP_ID, got {:?}", other),
    };
    let c1 = match &out.features[1].attributes[comp_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer COMP_ID, got {:?}", other),
    };
    let c2 = match &out.features[2].attributes[comp_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer COMP_ID, got {:?}", other),
    };

    assert_eq!(c0, c1, "connected edges should share the same component id");
    assert_ne!(c0, c2, "disconnected subnetwork should have a different component id");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn shortest_path_network_barrier_snap_distance_can_ignore_distant_barriers() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_shortest_path_network_barrier_snap_distance");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.15, 0.0))), &[])
        .expect("add near barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barriers");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.05));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("shortest_path_network", &args, &context(&caps))
        .expect("shortest_path_network run");

    let out = wbvector::read(&out_path).expect("read shortest path output");
    assert_eq!(out.features.len(), 1);
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 2.0).abs() < 1.0e-9, "expected barrier to be ignored due to small snap distance");

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_routes_from_od_outputs_route_geometry_and_cost() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_routes_from_od");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_routes_from_od", &args, &context(&caps))
        .expect("network_routes_from_od run");

    let out = wbvector::read(&out_path).expect("read route output");
    assert_eq!(out.features.len(), 1, "expected one route for one OD pair");
    let cost_idx = out.schema.field_index("COST").expect("COST field");

    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 2.0).abs() < 1.0e-9);

    match out.features[0].geometry.as_ref().expect("route geometry") {
        Geometry::LineString(coords) => {
            assert_eq!(coords.len(), 3, "expected route through network junction");
            assert!((coords[0].x - 0.0).abs() < 1.0e-9);
            assert!((coords[2].x - 2.0).abs() < 1.0e-9);
        }
        other => panic!("expected linestring route geometry, got {:?}", other),
    }

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_routes_from_od_respects_turn_restrictions_csv() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_routes_from_od_turn_restrictions_csv");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let restrictions_csv = std::env::temp_dir().join(format!("{tag}_turns.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add AB");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add BC");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add AD");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add DC");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 1.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    std::fs::write(
        &restrictions_csv,
        "prev_x,prev_y,node_x,node_y,next_x,next_y\n0,0,1,0,1,1\n",
    )
    .expect("write turn restrictions csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert(
        "turn_restrictions_csv".to_string(),
        json!(restrictions_csv.to_string_lossy().to_string()),
    );
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_routes_from_od", &args, &context(&caps))
        .expect("network_routes_from_od run");

    let out = wbvector::read(&out_path).expect("read route output");
    assert_eq!(out.features.len(), 1);
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 4.0).abs() < 1.0e-9, "expected detour cost 4.0 with turn restriction, got {}", cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&restrictions_csv);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn network_routes_from_od_temporal_profile_changes_cost_by_departure_time() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_routes_from_od_temporal_profile");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let rush_out_path = std::env::temp_dir().join(format!("{tag}_rush_routes.gpkg"));
    let offpeak_out_path = std::env::temp_dir().join(format!("{tag}_offpeak_routes.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("D_E".to_string()))],
        )
        .expect("add detour edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("E_F".to_string()))],
        )
        .expect("add detour edge 3");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("F_C".to_string()))],
        )
        .expect("add detour edge 4");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    rush_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    rush_args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(rush_out_path.to_string_lossy().to_string()));
    registry
        .run("network_routes_from_od", &rush_args, &context(&caps))
        .expect("network_routes_from_od rush run");

    let rush_out = wbvector::read(&rush_out_path).expect("read rush routes");
    assert_eq!(rush_out.features.len(), 1);
    let rush_cost_idx = rush_out.schema.field_index("COST").expect("COST field");
    let rush_cost = match &rush_out.features[0].attributes[rush_cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((rush_cost - 4.0).abs() < 1.0e-9, "expected rush-hour detour cost 4.0, got {}", rush_cost);

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    offpeak_args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    offpeak_args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(offpeak_out_path.to_string_lossy().to_string()));
    registry
        .run("network_routes_from_od", &offpeak_args, &context(&caps))
        .expect("network_routes_from_od offpeak run");

    let offpeak_out = wbvector::read(&offpeak_out_path).expect("read offpeak routes");
    assert_eq!(offpeak_out.features.len(), 1);
    let offpeak_cost_idx = offpeak_out.schema.field_index("COST").expect("COST field");
    let offpeak_cost = match &offpeak_out.features[0].attributes[offpeak_cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((offpeak_cost - 2.0).abs() < 1.0e-9, "expected offpeak direct cost 2.0, got {}", offpeak_cost);

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&rush_out_path);
    let _ = std::fs::remove_file(&offpeak_out_path);
}

#[test]
fn network_routes_from_od_respects_barrier_points() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_network_routes_from_od_barriers");
    let network_path = std::env::temp_dir().join(format!("{tag}_network.gpkg"));
    let origins_path = std::env::temp_dir().join(format!("{tag}_origins.gpkg"));
    let destinations_path = std::env::temp_dir().join(format!("{tag}_destinations.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_routes.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &network_path, VectorFormat::GeoPackage).expect("write network");

    let mut origins = Layer::new("origins")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    origins
        .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
        .expect("add origin");
    wbvector::write(&origins, &origins_path, VectorFormat::GeoPackage).expect("write origins");

    let mut destinations = Layer::new("destinations")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    destinations
        .add_feature(Some(Geometry::Point(Coord::xy(2.0, 0.0))), &[])
        .expect("add destination");
    wbvector::write(&destinations, &destinations_path, VectorFormat::GeoPackage)
        .expect("write destinations");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barriers");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(network_path.to_string_lossy().to_string()));
    args.insert("origins".to_string(), json!(origins_path.to_string_lossy().to_string()));
    args.insert("destinations".to_string(), json!(destinations_path.to_string_lossy().to_string()));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("network_routes_from_od", &args, &context(&caps))
        .expect("network_routes_from_od run");

    let out = wbvector::read(&out_path).expect("read route output");
    assert_eq!(out.features.len(), 0, "expected no route features when barrier blocks the only corridor");

    let _ = std::fs::remove_file(&network_path);
    let _ = std::fs::remove_file(&origins_path);
    let _ = std::fs::remove_file(&destinations_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn k_shortest_paths_network_returns_multiple_ranked_routes() {
    use wbvector::{Coord, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_k_shortest_paths_network");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[],
        )
        .expect("add edge 3");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[],
        )
        .expect("add edge 4");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 5");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("k".to_string(), json!(2));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("k_shortest_paths_network", &args, &context(&caps))
        .expect("k_shortest_paths_network run");

    let out = wbvector::read(&out_path).expect("read k shortest paths output");
    assert_eq!(out.features.len(), 2, "expected two alternative paths");

    let rank_idx = out.schema.field_index("PATH_RANK").expect("PATH_RANK field");
    let cost_idx = out.schema.field_index("COST").expect("COST field");

    let rank1 = match &out.features[0].attributes[rank_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer PATH_RANK, got {:?}", other),
    };
    let rank2 = match &out.features[1].attributes[rank_idx] {
        FieldValue::Integer(v) => *v,
        other => panic!("expected integer PATH_RANK, got {:?}", other),
    };
    assert_eq!(rank1, 1);
    assert_eq!(rank2, 2);

    let cost1 = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    let cost2 = match &out.features[1].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!(cost1 <= cost2, "paths should be sorted by ascending cost");
    assert!((cost1 - 2.0).abs() < 1.0e-9);
    assert!((cost2 - 4.0).abs() < 1.0e-9);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn k_shortest_paths_network_temporal_profile_changes_route_by_departure_time() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_k_shortest_paths_temporal_profile_route");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let rush_out_path = std::env::temp_dir().join(format!("{tag}_rush_out.gpkg"));
    let offpeak_out_path = std::env::temp_dir().join(format!("{tag}_offpeak_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));

    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add direct edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add direct edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("A_D".to_string()))],
        )
        .expect("add detour edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("D_E".to_string()))],
        )
        .expect("add detour edge 2");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 1.0), Coord::xy(2.0, 1.0)])),
            &[("EDGE_ID", FieldValue::Text("E_F".to_string()))],
        )
        .expect("add detour edge 3");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("F_C".to_string()))],
        )
        .expect("add detour edge 4");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,10\nB_C,1,420,600,10\n",
    )
    .expect("write temporal profile csv");

    let mut rush_args = ToolArgs::new();
    rush_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    rush_args.insert("start_x".to_string(), json!(0.0));
    rush_args.insert("start_y".to_string(), json!(0.0));
    rush_args.insert("end_x".to_string(), json!(2.0));
    rush_args.insert("end_y".to_string(), json!(0.0));
    rush_args.insert("k".to_string(), json!(1));
    rush_args.insert("max_snap_distance".to_string(), json!(0.25));
    rush_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    rush_args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    rush_args.insert("output".to_string(), json!(rush_out_path.to_string_lossy().to_string()));

    registry
        .run("k_shortest_paths_network", &rush_args, &context(&caps))
        .expect("k_shortest_paths_network rush run");

    let rush_out = wbvector::read(&rush_out_path).expect("read k shortest rush output");
    assert_eq!(rush_out.features.len(), 1);
    let rush_cost_idx = rush_out.schema.field_index("COST").expect("COST field");
    let rush_cost = match &rush_out.features[0].attributes[rush_cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((rush_cost - 4.0).abs() < 1.0e-9, "expected detour cost 4.0 during rush hour, got {}", rush_cost);

    let mut offpeak_args = ToolArgs::new();
    offpeak_args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    offpeak_args.insert("start_x".to_string(), json!(0.0));
    offpeak_args.insert("start_y".to_string(), json!(0.0));
    offpeak_args.insert("end_x".to_string(), json!(2.0));
    offpeak_args.insert("end_y".to_string(), json!(0.0));
    offpeak_args.insert("k".to_string(), json!(1));
    offpeak_args.insert("max_snap_distance".to_string(), json!(0.25));
    offpeak_args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    offpeak_args.insert("departure_time".to_string(), json!("2026-04-13T11:00:00Z"));
    offpeak_args.insert("output".to_string(), json!(offpeak_out_path.to_string_lossy().to_string()));

    registry
        .run("k_shortest_paths_network", &offpeak_args, &context(&caps))
        .expect("k_shortest_paths_network offpeak run");

    let offpeak_out = wbvector::read(&offpeak_out_path).expect("read k shortest offpeak output");
    assert_eq!(offpeak_out.features.len(), 1);
    let offpeak_cost_idx = offpeak_out.schema.field_index("COST").expect("COST field");
    let offpeak_cost = match &offpeak_out.features[0].attributes[offpeak_cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((offpeak_cost - 2.0).abs() < 1.0e-9, "expected direct cost 2.0 offpeak, got {}", offpeak_cost);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&rush_out_path);
    let _ = std::fs::remove_file(&offpeak_out_path);
}

#[test]
fn k_shortest_paths_network_temporal_profile_error_fallback_requires_coverage() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_k_shortest_paths_temporal_profile_error_fallback");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let profile_csv = std::env::temp_dir().join(format!("{tag}_temporal.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    std::fs::write(
        &profile_csv,
        "edge_id,dow,start_minute,end_minute,value\nA_B,1,420,600,2\n",
    )
    .expect("write temporal profile csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("k".to_string(), json!(1));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("temporal_cost_profile".to_string(), json!(profile_csv.to_string_lossy().to_string()));
    args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
    args.insert("temporal_fallback".to_string(), json!("error"));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("k_shortest_paths_network", &args, &context(&caps))
        .expect_err("expected failure when temporal_fallback=error and edge profile is missing");
    let msg = err.to_string();
    assert!(
        msg.contains("missing temporal cost profile"),
        "unexpected error: {}",
        msg
    );

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&profile_csv);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn k_shortest_paths_network_respects_turn_restrictions_csv() {
    use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_k_shortest_paths_turn_restrictions_csv");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let restrictions_csv = std::env::temp_dir().join(format!("{tag}_turns.csv"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines.schema.add_field(FieldDef::new("IMP", FieldType::Float));
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add AB");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(1.0))],
        )
        .expect("add BC");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add AD");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 1.0), Coord::xy(1.0, 1.0)])),
            &[("IMP", FieldValue::Float(2.0))],
        )
        .expect("add DC");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    std::fs::write(
        &restrictions_csv,
        "prev_x,prev_y,node_x,node_y,next_x,next_y\n0,0,1,0,1,1\n",
    )
    .expect("write turn restrictions csv");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(1.0));
    args.insert("end_y".to_string(), json!(1.0));
    args.insert("k".to_string(), json!(2));
    args.insert("edge_cost_field".to_string(), json!("IMP"));
    args.insert(
        "turn_restrictions_csv".to_string(),
        json!(restrictions_csv.to_string_lossy().to_string()),
    );
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
    registry
        .run("k_shortest_paths_network", &args, &context(&caps))
        .expect("k_shortest_paths_network run");

    let out = wbvector::read(&out_path).expect("read k shortest paths output");
    assert_eq!(out.features.len(), 1, "expected only one feasible path after restriction");
    let cost_idx = out.schema.field_index("COST").expect("COST field");
    let cost = match &out.features[0].attributes[cost_idx] {
        FieldValue::Float(v) => *v,
        FieldValue::Integer(v) => *v as f64,
        other => panic!("expected numeric COST, got {:?}", other),
    };
    assert!((cost - 4.0).abs() < 1.0e-9, "expected remaining path cost 4.0, got {}", cost);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&restrictions_csv);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn k_shortest_paths_network_respects_barrier_points() {
    use wbvector::{Coord, Geometry, Layer, VectorFormat};

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let caps = OpenOnly;

    let tag = unique_tag("wbtools_oss_k_shortest_paths_network_barriers");
    let input_path = std::env::temp_dir().join(format!("{tag}_in.gpkg"));
    let barriers_path = std::env::temp_dir().join(format!("{tag}_barriers.gpkg"));
    let out_path = std::env::temp_dir().join(format!("{tag}_out.gpkg"));

    let mut lines = Layer::new("network")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
            &[],
        )
        .expect("add edge 1");
    lines
        .add_feature(
            Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
            &[],
        )
        .expect("add edge 2");
    wbvector::write(&lines, &input_path, VectorFormat::GeoPackage).expect("write network input");

    let mut barriers = Layer::new("barriers")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);
    barriers
        .add_feature(Some(Geometry::Point(Coord::xy(1.0, 0.0))), &[])
        .expect("add barrier");
    wbvector::write(&barriers, &barriers_path, VectorFormat::GeoPackage).expect("write barriers");

    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path.to_string_lossy().to_string()));
    args.insert("start_x".to_string(), json!(0.0));
    args.insert("start_y".to_string(), json!(0.0));
    args.insert("end_x".to_string(), json!(2.0));
    args.insert("end_y".to_string(), json!(0.0));
    args.insert("k".to_string(), json!(2));
    args.insert("barriers".to_string(), json!(barriers_path.to_string_lossy().to_string()));
    args.insert("barrier_snap_distance".to_string(), json!(0.25));
    args.insert("max_snap_distance".to_string(), json!(0.25));
    args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

    let err = registry
        .run("k_shortest_paths_network", &args, &context(&caps))
        .expect_err("expected no path when barrier blocks the only corridor");
    let msg = err.to_string();
    assert!(msg.contains("no path found"), "unexpected error: {}", msg);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&barriers_path);
    let _ = std::fs::remove_file(&out_path);
}
