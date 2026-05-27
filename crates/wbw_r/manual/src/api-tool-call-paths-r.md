# Tool Call Paths (R)

This chapter maps each tool identifier to concrete R call surfaces and summarizes outputs.

Call pattern:

- Generated wrapper: s$tool_id(...)
- Generic execution: wbw_run_tool("tool_id", args = list(...), session = s)

Total tools: 747

| Tool ID | Category | Subcategory | Wrapper Call | Generic Call | Return Type | Output Summary |
|---|---|---|---|---|---|---|
| `abs` | `raster` | `general` | `s$abs(...)` | `wbw_run_tool("abs", args = list(...), session = s)` | `Raster` | Raster output |
| `accumulation_curvature` | `terrain` | `derivatives` | `s$accumulation_curvature(...)` | `wbw_run_tool("accumulation_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `adaptive_filter` | `remote_sensing` | `filters` | `s$adaptive_filter(...)` | `wbw_run_tool("adaptive_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `add` | `raster` | `overlay_math` | `s$add(...)` | `wbw_run_tool("add", args = list(...), session = s)` | `Raster` | Raster output |
| `add_field` | `vector` | `attribute_analysis` | `s$add_field(...)` | `wbw_run_tool("add_field", args = list(...), session = s)` | `Vector` | Vector output |
| `add_geometry_attributes` | `vector` | `attribute_analysis` | `s$add_geometry_attributes(...)` | `wbw_run_tool("add_geometry_attributes", args = list(...), session = s)` | `Vector` | Vector output |
| `add_point_coordinates_to_table` | `conversion` | `vector_table_io` | `s$add_point_coordinates_to_table(...)` | `wbw_run_tool("add_point_coordinates_to_table", args = list(...), session = s)` | `Vector` | Vector output |
| `aggregate_raster` | `raster` | `general` | `s$aggregate_raster(...)` | `wbw_run_tool("aggregate_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `anisotropic_diffusion_filter` | `remote_sensing` | `filters` | `s$anisotropic_diffusion_filter(...)` | `wbw_run_tool("anisotropic_diffusion_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `anova` | `raster` | `general` | `s$anova(...)` | `wbw_run_tool("anova", args = list(...), session = s)` | `str` | Report/path string output |
| `arccos` | `raster` | `general` | `s$arccos(...)` | `wbw_run_tool("arccos", args = list(...), session = s)` | `Raster` | Raster output |
| `arcosh` | `raster` | `general` | `s$arcosh(...)` | `wbw_run_tool("arcosh", args = list(...), session = s)` | `Raster` | Raster output |
| `arcsin` | `raster` | `general` | `s$arcsin(...)` | `wbw_run_tool("arcsin", args = list(...), session = s)` | `Raster` | Raster output |
| `arctan` | `raster` | `general` | `s$arctan(...)` | `wbw_run_tool("arctan", args = list(...), session = s)` | `Raster` | Raster output |
| `arsinh` | `raster` | `general` | `s$arsinh(...)` | `wbw_run_tool("arsinh", args = list(...), session = s)` | `Raster` | Raster output |
| `artanh` | `raster` | `general` | `s$artanh(...)` | `wbw_run_tool("artanh", args = list(...), session = s)` | `Raster` | Raster output |
| `ascii_to_las` | `lidar` | `io_management` | `s$ascii_to_las(...)` | `wbw_run_tool("ascii_to_las", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `aspect` | `terrain` | `derivatives` | `s$aspect(...)` | `wbw_run_tool("aspect", args = list(...), session = s)` | `Raster` | Raster output |
| `assess_route` | `terrain` | `general` | `s$assess_route(...)` | `wbw_run_tool("assess_route", args = list(...), session = s)` | `Vector` | Vector output |
| `assign_projection_lidar` | `projection_georeferencing` | `general` | `s$assign_projection_lidar(...)` | `wbw_run_tool("assign_projection_lidar", args = list(...), session = s)` | `Any` | See tool docs |
| `assign_projection_raster` | `projection_georeferencing` | `general` | `s$assign_projection_raster(...)` | `wbw_run_tool("assign_projection_raster", args = list(...), session = s)` | `Any` | See tool docs |
| `assign_projection_vector` | `projection_georeferencing` | `general` | `s$assign_projection_vector(...)` | `wbw_run_tool("assign_projection_vector", args = list(...), session = s)` | `Any` | See tool docs |
| `atan2` | `raster` | `general` | `s$atan2(...)` | `wbw_run_tool("atan2", args = list(...), session = s)` | `Raster` | Raster output |
| `attribute_correlation` | `vector` | `attribute_analysis` | `s$attribute_correlation(...)` | `wbw_run_tool("attribute_correlation", args = list(...), session = s)` | `str` | Report/path string output |
| `attribute_histogram` | `vector` | `attribute_analysis` | `s$attribute_histogram(...)` | `wbw_run_tool("attribute_histogram", args = list(...), session = s)` | `str` | Report/path string output |
| `attribute_scattergram` | `vector` | `attribute_analysis` | `s$attribute_scattergram(...)` | `wbw_run_tool("attribute_scattergram", args = list(...), session = s)` | `str` | Report/path string output |
| `average_flowpath_slope` | `hydrology` | `flow_routing` | `s$average_flowpath_slope(...)` | `wbw_run_tool("average_flowpath_slope", args = list(...), session = s)` | `Raster` | Raster output |
| `average_horizon_distance` | `terrain` | `visibility` | `s$average_horizon_distance(...)` | `wbw_run_tool("average_horizon_distance", args = list(...), session = s)` | `Raster` | Raster output |
| `average_normal_vector_angular_deviation` | `terrain` | `roughness_texture` | `s$average_normal_vector_angular_deviation(...)` | `wbw_run_tool("average_normal_vector_angular_deviation", args = list(...), session = s)` | `Raster` | Raster output |
| `average_overlay` | `raster` | `overlay_math` | `s$average_overlay(...)` | `wbw_run_tool("average_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `average_upslope_flowpath_length` | `hydrology` | `flow_routing` | `s$average_upslope_flowpath_length(...)` | `wbw_run_tool("average_upslope_flowpath_length", args = list(...), session = s)` | `Raster` | Raster output |
| `balance_contrast_enhancement` | `remote_sensing` | `enhancement_contrast` | `s$balance_contrast_enhancement(...)` | `wbw_run_tool("balance_contrast_enhancement", args = list(...), session = s)` | `Raster` | Raster output |
| `basins` | `hydrology` | `watersheds_basins` | `s$basins(...)` | `wbw_run_tool("basins", args = list(...), session = s)` | `Raster` | Raster output |
| `bilateral_filter` | `remote_sensing` | `filters` | `s$bilateral_filter(...)` | `wbw_run_tool("bilateral_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `block_maximum` | `raster` | `general` | `s$block_maximum(...)` | `wbw_run_tool("block_maximum", args = list(...), session = s)` | `Raster` | Raster output |
| `block_minimum` | `raster` | `general` | `s$block_minimum(...)` | `wbw_run_tool("block_minimum", args = list(...), session = s)` | `Raster` | Raster output |
| `bool_and` | `raster` | `overlay_math` | `s$bool_and(...)` | `wbw_run_tool("bool_and", args = list(...), session = s)` | `Raster` | Raster output |
| `bool_not` | `raster` | `overlay_math` | `s$bool_not(...)` | `wbw_run_tool("bool_not", args = list(...), session = s)` | `Raster` | Raster output |
| `bool_or` | `raster` | `overlay_math` | `s$bool_or(...)` | `wbw_run_tool("bool_or", args = list(...), session = s)` | `Raster` | Raster output |
| `bool_xor` | `raster` | `overlay_math` | `s$bool_xor(...)` | `wbw_run_tool("bool_xor", args = list(...), session = s)` | `Raster` | Raster output |
| `boundary_shape_complexity` | `raster` | `general` | `s$boundary_shape_complexity(...)` | `wbw_run_tool("boundary_shape_complexity", args = list(...), session = s)` | `Raster` | Raster output |
| `brdf_normalization` | `remote_sensing` | `radiometric_correction` | `s$brdf_normalization(...)` | `wbw_run_tool("brdf_normalization", args = list(...), session = s)` | `Any` | See tool docs |
| `brdf_surface_reflectance_consistency` | `remote_sensing` | `radiometric_correction` | `s$brdf_surface_reflectance_consistency(...)` | `wbw_run_tool("brdf_surface_reflectance_consistency", args = list(...), session = s)` | `tuple[Raster, Raster, Raster, str]` | Multiple outputs (tuple) |
| `breach_depressions_least_cost` | `hydrology` | `depressions_storage` | `s$breach_depressions_least_cost(...)` | `wbw_run_tool("breach_depressions_least_cost", args = list(...), session = s)` | `Raster` | Raster output |
| `breach_single_cell_pits` | `hydrology` | `depressions_storage` | `s$breach_single_cell_pits(...)` | `wbw_run_tool("breach_single_cell_pits", args = list(...), session = s)` | `Raster` | Raster output |
| `breakline_mapping` | `terrain` | `general` | `s$breakline_mapping(...)` | `wbw_run_tool("breakline_mapping", args = list(...), session = s)` | `Vector` | Vector output |
| `buffer_raster` | `raster` | `distance_cost` | `s$buffer_raster(...)` | `wbw_run_tool("buffer_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `build_network_topology` | `vector` | `network_analysis` | `s$build_network_topology(...)` | `wbw_run_tool("build_network_topology", args = list(...), session = s)` | `Any` | See tool docs |
| `build_object_hierarchy_multiscale` | `remote_sensing` | `obia` | `s$build_object_hierarchy_multiscale(...)` | `wbw_run_tool("build_object_hierarchy_multiscale", args = list(...), session = s)` | `str` | Report/path string output |
| `burn_streams` | `hydrology` | `depressions_storage` | `s$burn_streams(...)` | `wbw_run_tool("burn_streams", args = list(...), session = s)` | `Raster` | Raster output |
| `burn_streams_at_roads` | `hydrology` | `depressions_storage` | `s$burn_streams_at_roads(...)` | `wbw_run_tool("burn_streams_at_roads", args = list(...), session = s)` | `Raster` | Raster output |
| `canny_edge_detection` | `remote_sensing` | `edge_feature_detection` | `s$canny_edge_detection(...)` | `wbw_run_tool("canny_edge_detection", args = list(...), session = s)` | `Raster` | Raster output |
| `carbon_sequestration_verification_audit` | `terrain` | `workflow_products` | `s$carbon_sequestration_verification_audit(...)` | `wbw_run_tool("carbon_sequestration_verification_audit", args = list(...), session = s)` | `Any` | See tool docs |
| `casorati_curvature` | `terrain` | `derivatives` | `s$casorati_curvature(...)` | `wbw_run_tool("casorati_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `ceil` | `raster` | `general` | `s$ceil(...)` | `wbw_run_tool("ceil", args = list(...), session = s)` | `Raster` | Raster output |
| `centroid_raster` | `raster` | `general` | `s$centroid_raster(...)` | `wbw_run_tool("centroid_raster", args = list(...), session = s)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `centroid_vector` | `vector` | `geometry_processing` | `s$centroid_vector(...)` | `wbw_run_tool("centroid_vector", args = list(...), session = s)` | `Vector` | Vector output |
| `change_vector_analysis` | `remote_sensing` | `change_detection` | `s$change_vector_analysis(...)` | `wbw_run_tool("change_vector_analysis", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `circular_variance_of_aspect` | `terrain` | `roughness_texture` | `s$circular_variance_of_aspect(...)` | `wbw_run_tool("circular_variance_of_aspect", args = list(...), session = s)` | `Raster` | Raster output |
| `classify_buildings_in_lidar` | `lidar` | `filtering_classification` | `s$classify_buildings_in_lidar(...)` | `wbw_run_tool("classify_buildings_in_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `classify_lidar` | `lidar` | `filtering_classification` | `s$classify_lidar(...)` | `wbw_run_tool("classify_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `classify_objects_ensemble_pro` | `remote_sensing` | `obia` | `s$classify_objects_ensemble_pro(...)` | `wbw_run_tool("classify_objects_ensemble_pro", args = list(...), session = s)` | `str` | Report/path string output |
| `classify_objects_random_forest` | `remote_sensing` | `obia` | `s$classify_objects_random_forest(...)` | `wbw_run_tool("classify_objects_random_forest", args = list(...), session = s)` | `str` | Report/path string output |
| `classify_objects_rules_basic` | `remote_sensing` | `obia` | `s$classify_objects_rules_basic(...)` | `wbw_run_tool("classify_objects_rules_basic", args = list(...), session = s)` | `str` | Report/path string output |
| `classify_objects_rules_hierarchical` | `remote_sensing` | `obia` | `s$classify_objects_rules_hierarchical(...)` | `wbw_run_tool("classify_objects_rules_hierarchical", args = list(...), session = s)` | `str` | Report/path string output |
| `classify_objects_svm` | `remote_sensing` | `obia` | `s$classify_objects_svm(...)` | `wbw_run_tool("classify_objects_svm", args = list(...), session = s)` | `str` | Report/path string output |
| `classify_overlap_points` | `lidar` | `filtering_classification` | `s$classify_overlap_points(...)` | `wbw_run_tool("classify_overlap_points", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `clean_vector` | `conversion` | `vector_table_io` | `s$clean_vector(...)` | `wbw_run_tool("clean_vector", args = list(...), session = s)` | `Vector` | Vector output |
| `clip` | `vector` | `overlay_analysis` | `s$clip(...)` | `wbw_run_tool("clip", args = list(...), session = s)` | `Vector` | Vector output |
| `clip_lidar_to_polygon` | `lidar` | `filtering_classification` | `s$clip_lidar_to_polygon(...)` | `wbw_run_tool("clip_lidar_to_polygon", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `clip_raster_to_polygon` | `raster` | `general` | `s$clip_raster_to_polygon(...)` | `wbw_run_tool("clip_raster_to_polygon", args = list(...), session = s)` | `Raster` | Raster output |
| `closest_facility_network` | `vector` | `network_analysis` | `s$closest_facility_network(...)` | `wbw_run_tool("closest_facility_network", args = list(...), session = s)` | `Vector` | Vector output |
| `closing` | `remote_sensing` | `filters` | `s$closing(...)` | `wbw_run_tool("closing", args = list(...), session = s)` | `Raster` | Raster output |
| `cloude_pottier_decomposition` | `remote_sensing` | `sar` | `s$cloude_pottier_decomposition(...)` | `wbw_run_tool("cloude_pottier_decomposition", args = list(...), session = s)` | `Any` | See tool docs |
| `clump` | `raster` | `general` | `s$clump(...)` | `wbw_run_tool("clump", args = list(...), session = s)` | `Raster` | Raster output |
| `colourize_based_on_class` | `lidar` | `analysis_metrics` | `s$colourize_based_on_class(...)` | `wbw_run_tool("colourize_based_on_class", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `colourize_based_on_point_returns` | `lidar` | `analysis_metrics` | `s$colourize_based_on_point_returns(...)` | `wbw_run_tool("colourize_based_on_point_returns", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `compactness_ratio` | `vector` | `shape_metrics` | `s$compactness_ratio(...)` | `wbw_run_tool("compactness_ratio", args = list(...), session = s)` | `Vector` | Vector output |
| `concave_hull` | `vector` | `geometry_processing` | `s$concave_hull(...)` | `wbw_run_tool("concave_hull", args = list(...), session = s)` | `Vector` | Vector output |
| `conditional_evaluation` | `raster` | `reclass_mask` | `s$conditional_evaluation(...)` | `wbw_run_tool("conditional_evaluation", args = list(...), session = s)` | `Raster` | Raster output |
| `conservative_smoothing_filter` | `remote_sensing` | `filters` | `s$conservative_smoothing_filter(...)` | `wbw_run_tool("conservative_smoothing_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `construct_vector_tin` | `vector` | `sampling_gridding` | `s$construct_vector_tin(...)` | `wbw_run_tool("construct_vector_tin", args = list(...), session = s)` | `Vector` | Vector output |
| `continuum_removal` | `remote_sensing` | `spectral_analytics` | `s$continuum_removal(...)` | `wbw_run_tool("continuum_removal", args = list(...), session = s)` | `Any` | See tool docs |
| `contours_from_points` | `vector` | `sampling_gridding` | `s$contours_from_points(...)` | `wbw_run_tool("contours_from_points", args = list(...), session = s)` | `Vector` | Vector output |
| `contours_from_raster` | `vector` | `sampling_gridding` | `s$contours_from_raster(...)` | `wbw_run_tool("contours_from_raster", args = list(...), session = s)` | `Vector` | Vector output |
| `convergence_index` | `terrain` | `general` | `s$convergence_index(...)` | `wbw_run_tool("convergence_index", args = list(...), session = s)` | `Raster` | Raster output |
| `convert_nodata_to_zero` | `conversion` | `raster_vector_conversion` | `s$convert_nodata_to_zero(...)` | `wbw_run_tool("convert_nodata_to_zero", args = list(...), session = s)` | `Raster` | Raster output |
| `corner_detection` | `remote_sensing` | `edge_feature_detection` | `s$corner_detection(...)` | `wbw_run_tool("corner_detection", args = list(...), session = s)` | `Raster` | Raster output |
| `correct_vignetting` | `remote_sensing` | `radiometric_correction` | `s$correct_vignetting(...)` | `wbw_run_tool("correct_vignetting", args = list(...), session = s)` | `Raster` | Raster output |
| `corridor_mapping_intelligence` | `terrain` | `workflow_products` | `s$corridor_mapping_intelligence(...)` | `wbw_run_tool("corridor_mapping_intelligence", args = list(...), session = s)` | `Any` | See tool docs |
| `cos` | `raster` | `general` | `s$cos(...)` | `wbw_run_tool("cos", args = list(...), session = s)` | `Raster` | Raster output |
| `cosh` | `raster` | `general` | `s$cosh(...)` | `wbw_run_tool("cosh", args = list(...), session = s)` | `Raster` | Raster output |
| `cost_allocation` | `raster` | `distance_cost` | `s$cost_allocation(...)` | `wbw_run_tool("cost_allocation", args = list(...), session = s)` | `Raster` | Raster output |
| `cost_distance` | `raster` | `distance_cost` | `s$cost_distance(...)` | `wbw_run_tool("cost_distance", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `cost_pathway` | `raster` | `distance_cost` | `s$cost_pathway(...)` | `wbw_run_tool("cost_pathway", args = list(...), session = s)` | `Raster` | Raster output |
| `count_if` | `raster` | `overlay_math` | `s$count_if(...)` | `wbw_run_tool("count_if", args = list(...), session = s)` | `Raster` | Raster output |
| `create_colour_composite` | `remote_sensing` | `enhancement_contrast` | `s$create_colour_composite(...)` | `wbw_run_tool("create_colour_composite", args = list(...), session = s)` | `Raster` | Raster output |
| `create_plane` | `raster` | `general` | `s$create_plane(...)` | `wbw_run_tool("create_plane", args = list(...), session = s)` | `Raster` | Raster output |
| `crispness_index` | `raster` | `general` | `s$crispness_index(...)` | `wbw_run_tool("crispness_index", args = list(...), session = s)` | `str` | Report/path string output |
| `cross_tabulation` | `raster` | `general` | `s$cross_tabulation(...)` | `wbw_run_tool("cross_tabulation", args = list(...), session = s)` | `str` | Report/path string output |
| `csv_points_to_vector` | `conversion` | `vector_table_io` | `s$csv_points_to_vector(...)` | `wbw_run_tool("csv_points_to_vector", args = list(...), session = s)` | `Vector` | Vector output |
| `cumulative_distribution` | `raster` | `general` | `s$cumulative_distribution(...)` | `wbw_run_tool("cumulative_distribution", args = list(...), session = s)` | `Raster` | Raster output |
| `curvedness` | `terrain` | `derivatives` | `s$curvedness(...)` | `wbw_run_tool("curvedness", args = list(...), session = s)` | `Raster` | Raster output |
| `d8_flow_accum` | `hydrology` | `flow_routing` | `s$d8_flow_accum(...)` | `wbw_run_tool("d8_flow_accum", args = list(...), session = s)` | `Raster` | Raster output |
| `d8_mass_flux` | `hydrology` | `flow_routing` | `s$d8_mass_flux(...)` | `wbw_run_tool("d8_mass_flux", args = list(...), session = s)` | `Raster` | Raster output |
| `d8_pointer` | `hydrology` | `flow_routing` | `s$d8_pointer(...)` | `wbw_run_tool("d8_pointer", args = list(...), session = s)` | `Raster` | Raster output |
| `dark_object_subtraction` | `remote_sensing` | `radiometric_correction` | `s$dark_object_subtraction(...)` | `wbw_run_tool("dark_object_subtraction", args = list(...), session = s)` | `Any` | See tool docs |
| `dbscan` | `raster` | `general` | `s$dbscan(...)` | `wbw_run_tool("dbscan", args = list(...), session = s)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `decrement` | `raster` | `general` | `s$decrement(...)` | `wbw_run_tool("decrement", args = list(...), session = s)` | `Raster` | Raster output |
| `delete_field` | `vector` | `attribute_analysis` | `s$delete_field(...)` | `wbw_run_tool("delete_field", args = list(...), session = s)` | `Vector` | Vector output |
| `dem_void_filling` | `terrain` | `general` | `s$dem_void_filling(...)` | `wbw_run_tool("dem_void_filling", args = list(...), session = s)` | `Raster` | Raster output |
| `densify_features` | `vector` | `geometry_processing` | `s$densify_features(...)` | `wbw_run_tool("densify_features", args = list(...), session = s)` | `Vector` | Vector output |
| `depth_in_sink` | `hydrology` | `depressions_storage` | `s$depth_in_sink(...)` | `wbw_run_tool("depth_in_sink", args = list(...), session = s)` | `Raster` | Raster output |
| `depth_to_water` | `hydrology` | `hydrologic_indices` | `s$depth_to_water(...)` | `wbw_run_tool("depth_to_water", args = list(...), session = s)` | `Raster` | Raster output |
| `deviation_from_mean_elevation` | `terrain` | `general` | `s$deviation_from_mean_elevation(...)` | `wbw_run_tool("deviation_from_mean_elevation", args = list(...), session = s)` | `Raster` | Raster output |
| `deviation_from_regional_direction` | `vector` | `shape_metrics` | `s$deviation_from_regional_direction(...)` | `wbw_run_tool("deviation_from_regional_direction", args = list(...), session = s)` | `Vector` | Vector output |
| `diff_of_gaussians_filter` | `remote_sensing` | `filters` | `s$diff_of_gaussians_filter(...)` | `wbw_run_tool("diff_of_gaussians_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `difference` | `vector` | `overlay_analysis` | `s$difference(...)` | `wbw_run_tool("difference", args = list(...), session = s)` | `Vector` | Vector output |
| `difference_curvature` | `terrain` | `derivatives` | `s$difference_curvature(...)` | `wbw_run_tool("difference_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `difference_from_mean_elevation` | `terrain` | `general` | `s$difference_from_mean_elevation(...)` | `wbw_run_tool("difference_from_mean_elevation", args = list(...), session = s)` | `Raster` | Raster output |
| `dinf_flow_accum` | `hydrology` | `flow_routing` | `s$dinf_flow_accum(...)` | `wbw_run_tool("dinf_flow_accum", args = list(...), session = s)` | `Raster` | Raster output |
| `dinf_mass_flux` | `hydrology` | `flow_routing` | `s$dinf_mass_flux(...)` | `wbw_run_tool("dinf_mass_flux", args = list(...), session = s)` | `Raster` | Raster output |
| `dinf_pointer` | `hydrology` | `flow_routing` | `s$dinf_pointer(...)` | `wbw_run_tool("dinf_pointer", args = list(...), session = s)` | `Raster` | Raster output |
| `direct_decorrelation_stretch` | `remote_sensing` | `enhancement_contrast` | `s$direct_decorrelation_stretch(...)` | `wbw_run_tool("direct_decorrelation_stretch", args = list(...), session = s)` | `Raster` | Raster output |
| `directional_relief` | `terrain` | `general` | `s$directional_relief(...)` | `wbw_run_tool("directional_relief", args = list(...), session = s)` | `Raster` | Raster output |
| `dissolve` | `vector` | `overlay_analysis` | `s$dissolve(...)` | `wbw_run_tool("dissolve", args = list(...), session = s)` | `Vector` | Vector output |
| `distance_to_outlet` | `hydrology` | `hydrologic_indices` | `s$distance_to_outlet(...)` | `wbw_run_tool("distance_to_outlet", args = list(...), session = s)` | `Raster` | Raster output |
| `diversity_filter` | `remote_sensing` | `filters` | `s$diversity_filter(...)` | `wbw_run_tool("diversity_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `divide` | `raster` | `overlay_math` | `s$divide(...)` | `wbw_run_tool("divide", args = list(...), session = s)` | `Raster` | Raster output |
| `dn_to_toa_reflectance` | `remote_sensing` | `radiometric_correction` | `s$dn_to_toa_reflectance(...)` | `wbw_run_tool("dn_to_toa_reflectance", args = list(...), session = s)` | `Any` | See tool docs |
| `download_osm_vector` | `vector` | `online_data` | `s$download_osm_vector(...)` | `wbw_run_tool("download_osm_vector", args = list(...), session = s)` | `Any` | See tool docs |
| `downslope_distance_to_stream` | `hydrology` | `hydrologic_indices` | `s$downslope_distance_to_stream(...)` | `wbw_run_tool("downslope_distance_to_stream", args = list(...), session = s)` | `Raster` | Raster output |
| `downslope_flowpath_length` | `hydrology` | `flow_routing` | `s$downslope_flowpath_length(...)` | `wbw_run_tool("downslope_flowpath_length", args = list(...), session = s)` | `Raster` | Raster output |
| `downslope_index` | `hydrology` | `hydrologic_indices` | `s$downslope_index(...)` | `wbw_run_tool("downslope_index", args = list(...), session = s)` | `Raster` | Raster output |
| `edge_contamination` | `hydrology` | `hydrologic_indices` | `s$edge_contamination(...)` | `wbw_run_tool("edge_contamination", args = list(...), session = s)` | `Raster` | Raster output |
| `edge_density` | `terrain` | `roughness_texture` | `s$edge_density(...)` | `wbw_run_tool("edge_density", args = list(...), session = s)` | `Raster` | Raster output |
| `edge_preserving_mean_filter` | `remote_sensing` | `filters` | `s$edge_preserving_mean_filter(...)` | `wbw_run_tool("edge_preserving_mean_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `edge_proportion` | `raster` | `general` | `s$edge_proportion(...)` | `wbw_run_tool("edge_proportion", args = list(...), session = s)` | `Raster` | Raster output |
| `elev_above_pit` | `terrain` | `general` | `s$elev_above_pit(...)` | `wbw_run_tool("elev_above_pit", args = list(...), session = s)` | `Raster` | Raster output |
| `elev_above_pit_dist` | `terrain` | `general` | `s$elev_above_pit_dist(...)` | `wbw_run_tool("elev_above_pit_dist", args = list(...), session = s)` | `Raster` | Raster output |
| `elev_relative_to_min_max` | `terrain` | `landform_indices` | `s$elev_relative_to_min_max(...)` | `wbw_run_tool("elev_relative_to_min_max", args = list(...), session = s)` | `Raster` | Raster output |
| `elev_relative_to_watershed_min_max` | `hydrology` | `hydrologic_indices` | `s$elev_relative_to_watershed_min_max(...)` | `wbw_run_tool("elev_relative_to_watershed_min_max", args = list(...), session = s)` | `Raster` | Raster output |
| `elevation_above_stream` | `hydrology` | `hydrologic_indices` | `s$elevation_above_stream(...)` | `wbw_run_tool("elevation_above_stream", args = list(...), session = s)` | `Raster` | Raster output |
| `elevation_above_stream_euclidean` | `hydrology` | `hydrologic_indices` | `s$elevation_above_stream_euclidean(...)` | `wbw_run_tool("elevation_above_stream_euclidean", args = list(...), session = s)` | `Raster` | Raster output |
| `elevation_percentile` | `terrain` | `general` | `s$elevation_percentile(...)` | `wbw_run_tool("elevation_percentile", args = list(...), session = s)` | `Raster` | Raster output |
| `eliminate_coincident_points` | `vector` | `geometry_processing` | `s$eliminate_coincident_points(...)` | `wbw_run_tool("eliminate_coincident_points", args = list(...), session = s)` | `Vector` | Vector output |
| `elongation_ratio` | `vector` | `shape_metrics` | `s$elongation_ratio(...)` | `wbw_run_tool("elongation_ratio", args = list(...), session = s)` | `Vector` | Vector output |
| `embankment_mapping` | `terrain` | `general` | `s$embankment_mapping(...)` | `wbw_run_tool("embankment_mapping", args = list(...), session = s)` | `tuple[Raster, Raster | None]` | Multiple outputs (tuple) |
| `emboss_filter` | `remote_sensing` | `filters` | `s$emboss_filter(...)` | `wbw_run_tool("emboss_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `emergency_scenario_routing_and_accessibility_simulator` | `vector` | `network_analysis` | `s$emergency_scenario_routing_and_accessibility_simulator(...)` | `wbw_run_tool("emergency_scenario_routing_and_accessibility_simulator", args = list(...), session = s)` | `Any` | See tool docs |
| `enhanced_lee_filter` | `remote_sensing` | `sar` | `s$enhanced_lee_filter(...)` | `wbw_run_tool("enhanced_lee_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `equal_to` | `raster` | `general` | `s$equal_to(...)` | `wbw_run_tool("equal_to", args = list(...), session = s)` | `Raster` | Raster output |
| `erase` | `vector` | `overlay_analysis` | `s$erase(...)` | `wbw_run_tool("erase", args = list(...), session = s)` | `Vector` | Vector output |
| `erase_polygon_from_lidar` | `lidar` | `filtering_classification` | `s$erase_polygon_from_lidar(...)` | `wbw_run_tool("erase_polygon_from_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `erase_polygon_from_raster` | `raster` | `general` | `s$erase_polygon_from_raster(...)` | `wbw_run_tool("erase_polygon_from_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `euclidean_allocation` | `raster` | `distance_cost` | `s$euclidean_allocation(...)` | `wbw_run_tool("euclidean_allocation", args = list(...), session = s)` | `Raster` | Raster output |
| `euclidean_distance` | `raster` | `distance_cost` | `s$euclidean_distance(...)` | `wbw_run_tool("euclidean_distance", args = list(...), session = s)` | `Raster` | Raster output |
| `evaluate_object_classification_accuracy` | `remote_sensing` | `obia` | `s$evaluate_object_classification_accuracy(...)` | `wbw_run_tool("evaluate_object_classification_accuracy", args = list(...), session = s)` | `str` | Report/path string output |
| `evaluate_segmentation_quality_pro` | `remote_sensing` | `obia` | `s$evaluate_segmentation_quality_pro(...)` | `wbw_run_tool("evaluate_segmentation_quality_pro", args = list(...), session = s)` | `str` | Report/path string output |
| `evaluate_training_sites` | `remote_sensing` | `classification` | `s$evaluate_training_sites(...)` | `wbw_run_tool("evaluate_training_sites", args = list(...), session = s)` | `str` | Report/path string output |
| `exp` | `raster` | `general` | `s$exp(...)` | `wbw_run_tool("exp", args = list(...), session = s)` | `Raster` | Raster output |
| `exp2` | `raster` | `general` | `s$exp2(...)` | `wbw_run_tool("exp2", args = list(...), session = s)` | `Raster` | Raster output |
| `export_table_to_csv` | `conversion` | `vector_table_io` | `s$export_table_to_csv(...)` | `wbw_run_tool("export_table_to_csv", args = list(...), session = s)` | `str` | Report/path string output |
| `exposure_towards_wind_flux` | `terrain` | `general` | `s$exposure_towards_wind_flux(...)` | `wbw_run_tool("exposure_towards_wind_flux", args = list(...), session = s)` | `Raster` | Raster output |
| `extend_vector_lines` | `vector` | `geometry_processing` | `s$extend_vector_lines(...)` | `wbw_run_tool("extend_vector_lines", args = list(...), session = s)` | `Vector` | Vector output |
| `extract_by_attribute` | `vector` | `attribute_analysis` | `s$extract_by_attribute(...)` | `wbw_run_tool("extract_by_attribute", args = list(...), session = s)` | `Vector` | Vector output |
| `extract_nodes` | `vector` | `sampling_gridding` | `s$extract_nodes(...)` | `wbw_run_tool("extract_nodes", args = list(...), session = s)` | `Vector` | Vector output |
| `extract_raster_values_at_points` | `vector` | `sampling_gridding` | `s$extract_raster_values_at_points(...)` | `wbw_run_tool("extract_raster_values_at_points", args = list(...), session = s)` | `tuple[Vector, str]` | Multiple outputs (tuple) |
| `extract_streams` | `streams` | `network_extraction` | `s$extract_streams(...)` | `wbw_run_tool("extract_streams", args = list(...), session = s)` | `Raster` | Raster output |
| `extract_valleys` | `streams` | `network_extraction` | `s$extract_valleys(...)` | `wbw_run_tool("extract_valleys", args = list(...), session = s)` | `Raster` | Raster output |
| `false_colour_composite` | `remote_sensing` | `enhancement_contrast` | `s$false_colour_composite(...)` | `wbw_run_tool("false_colour_composite", args = list(...), session = s)` | `Any` | See tool docs |
| `farthest_channel_head` | `streams` | `longitudinal_analysis` | `s$farthest_channel_head(...)` | `wbw_run_tool("farthest_channel_head", args = list(...), session = s)` | `Raster` | Raster output |
| `fast_almost_gaussian_filter` | `remote_sensing` | `filters` | `s$fast_almost_gaussian_filter(...)` | `wbw_run_tool("fast_almost_gaussian_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `fd8_flow_accum` | `hydrology` | `flow_routing` | `s$fd8_flow_accum(...)` | `wbw_run_tool("fd8_flow_accum", args = list(...), session = s)` | `Raster` | Raster output |
| `fd8_pointer` | `hydrology` | `flow_routing` | `s$fd8_pointer(...)` | `wbw_run_tool("fd8_pointer", args = list(...), session = s)` | `Raster` | Raster output |
| `feature_preserving_smoothing` | `terrain` | `general` | `s$feature_preserving_smoothing(...)` | `wbw_run_tool("feature_preserving_smoothing", args = list(...), session = s)` | `Raster` | Raster output |
| `feature_preserving_smoothing_multiscale` | `terrain` | `general` | `s$feature_preserving_smoothing_multiscale(...)` | `wbw_run_tool("feature_preserving_smoothing_multiscale", args = list(...), session = s)` | `Any` | See tool docs |
| `fetch_analysis` | `terrain` | `general` | `s$fetch_analysis(...)` | `wbw_run_tool("fetch_analysis", args = list(...), session = s)` | `Raster` | Raster output |
| `fft_random_field` | `raster` | `general` | `s$fft_random_field(...)` | `wbw_run_tool("fft_random_field", args = list(...), session = s)` | `Raster` | Raster output |
| `field_calculator` | `vector` | `attribute_analysis` | `s$field_calculator(...)` | `wbw_run_tool("field_calculator", args = list(...), session = s)` | `Vector` | Vector output |
| `field_trafficability_and_operation_planning` | `precision_agriculture` | `general` | `s$field_trafficability_and_operation_planning(...)` | `wbw_run_tool("field_trafficability_and_operation_planning", args = list(...), session = s)` | `Any` | See tool docs |
| `fill_burn` | `hydrology` | `depressions_storage` | `s$fill_burn(...)` | `wbw_run_tool("fill_burn", args = list(...), session = s)` | `Raster` | Raster output |
| `fill_depressions` | `hydrology` | `depressions_storage` | `s$fill_depressions(...)` | `wbw_run_tool("fill_depressions", args = list(...), session = s)` | `Raster` | Raster output |
| `fill_depressions_planchon_and_darboux` | `hydrology` | `depressions_storage` | `s$fill_depressions_planchon_and_darboux(...)` | `wbw_run_tool("fill_depressions_planchon_and_darboux", args = list(...), session = s)` | `Raster` | Raster output |
| `fill_depressions_wang_and_liu` | `hydrology` | `depressions_storage` | `s$fill_depressions_wang_and_liu(...)` | `wbw_run_tool("fill_depressions_wang_and_liu", args = list(...), session = s)` | `Raster` | Raster output |
| `fill_missing_data` | `terrain` | `general` | `s$fill_missing_data(...)` | `wbw_run_tool("fill_missing_data", args = list(...), session = s)` | `Raster` | Raster output |
| `fill_pits` | `hydrology` | `depressions_storage` | `s$fill_pits(...)` | `wbw_run_tool("fill_pits", args = list(...), session = s)` | `Raster` | Raster output |
| `filter_lidar` | `lidar` | `filtering_classification` | `s$filter_lidar(...)` | `wbw_run_tool("filter_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `filter_lidar_by_percentile` | `lidar` | `filtering_classification` | `s$filter_lidar_by_percentile(...)` | `wbw_run_tool("filter_lidar_by_percentile", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `filter_lidar_by_reference_surface` | `lidar` | `filtering_classification` | `s$filter_lidar_by_reference_surface(...)` | `wbw_run_tool("filter_lidar_by_reference_surface", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `filter_lidar_classes` | `lidar` | `filtering_classification` | `s$filter_lidar_classes(...)` | `wbw_run_tool("filter_lidar_classes", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `filter_lidar_noise` | `lidar` | `filtering_classification` | `s$filter_lidar_noise(...)` | `wbw_run_tool("filter_lidar_noise", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `filter_lidar_scan_angles` | `lidar` | `filtering_classification` | `s$filter_lidar_scan_angles(...)` | `wbw_run_tool("filter_lidar_scan_angles", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `filter_raster_features_by_area` | `raster` | `general` | `s$filter_raster_features_by_area(...)` | `wbw_run_tool("filter_raster_features_by_area", args = list(...), session = s)` | `Raster` | Raster output |
| `filter_vector_features_by_area` | `vector` | `attribute_analysis` | `s$filter_vector_features_by_area(...)` | `wbw_run_tool("filter_vector_features_by_area", args = list(...), session = s)` | `Vector` | Vector output |
| `find_flightline_edge_points` | `lidar` | `analysis_metrics` | `s$find_flightline_edge_points(...)` | `wbw_run_tool("find_flightline_edge_points", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `find_lowest_or_highest_points` | `vector` | `sampling_gridding` | `s$find_lowest_or_highest_points(...)` | `wbw_run_tool("find_lowest_or_highest_points", args = list(...), session = s)` | `Vector` | Vector output |
| `find_main_stem` | `streams` | `longitudinal_analysis` | `s$find_main_stem(...)` | `wbw_run_tool("find_main_stem", args = list(...), session = s)` | `Raster` | Raster output |
| `find_noflow_cells` | `hydrology` | `hydrologic_indices` | `s$find_noflow_cells(...)` | `wbw_run_tool("find_noflow_cells", args = list(...), session = s)` | `Raster` | Raster output |
| `find_parallel_flow` | `hydrology` | `hydrologic_indices` | `s$find_parallel_flow(...)` | `wbw_run_tool("find_parallel_flow", args = list(...), session = s)` | `Raster` | Raster output |
| `find_patch_edge_cells` | `raster` | `general` | `s$find_patch_edge_cells(...)` | `wbw_run_tool("find_patch_edge_cells", args = list(...), session = s)` | `Raster` | Raster output |
| `find_ridges` | `terrain` | `general` | `s$find_ridges(...)` | `wbw_run_tool("find_ridges", args = list(...), session = s)` | `Raster` | Raster output |
| `fix_dangling_arcs` | `conversion` | `geometry_topology` | `s$fix_dangling_arcs(...)` | `wbw_run_tool("fix_dangling_arcs", args = list(...), session = s)` | `Vector` | Vector output |
| `flatten_lakes` | `hydrology` | `depressions_storage` | `s$flatten_lakes(...)` | `wbw_run_tool("flatten_lakes", args = list(...), session = s)` | `Raster` | Raster output |
| `fleet_routing_and_dispatch_optimizer` | `vector` | `network_analysis` | `s$fleet_routing_and_dispatch_optimizer(...)` | `wbw_run_tool("fleet_routing_and_dispatch_optimizer", args = list(...), session = s)` | `Any` | See tool docs |
| `flightline_overlap` | `lidar` | `interpolation_gridding` | `s$flightline_overlap(...)` | `wbw_run_tool("flightline_overlap", args = list(...), session = s)` | `Raster` | Raster output |
| `flip_image` | `remote_sensing` | `filters` | `s$flip_image(...)` | `wbw_run_tool("flip_image", args = list(...), session = s)` | `Raster` | Raster output |
| `flood_order` | `hydrology` | `watersheds_basins` | `s$flood_order(...)` | `wbw_run_tool("flood_order", args = list(...), session = s)` | `Raster` | Raster output |
| `floor` | `raster` | `general` | `s$floor(...)` | `wbw_run_tool("floor", args = list(...), session = s)` | `Raster` | Raster output |
| `flow_accum_full_workflow` | `hydrology` | `flow_routing` | `s$flow_accum_full_workflow(...)` | `wbw_run_tool("flow_accum_full_workflow", args = list(...), session = s)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `flow_length_diff` | `hydrology` | `flow_routing` | `s$flow_length_diff(...)` | `wbw_run_tool("flow_length_diff", args = list(...), session = s)` | `Raster` | Raster output |
| `forestry_structure_and_biomass_intelligence` | `terrain` | `workflow_products` | `s$forestry_structure_and_biomass_intelligence(...)` | `wbw_run_tool("forestry_structure_and_biomass_intelligence", args = list(...), session = s)` | `Any` | See tool docs |
| `frangi_filter` | `remote_sensing` | `filters` | `s$frangi_filter(...)` | `wbw_run_tool("frangi_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `freeman_durden_decomposition` | `remote_sensing` | `sar` | `s$freeman_durden_decomposition(...)` | `wbw_run_tool("freeman_durden_decomposition", args = list(...), session = s)` | `Any` | See tool docs |
| `frost_filter` | `remote_sensing` | `sar` | `s$frost_filter(...)` | `wbw_run_tool("frost_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `fuzzy_knn_classification` | `remote_sensing` | `classification` | `s$fuzzy_knn_classification(...)` | `wbw_run_tool("fuzzy_knn_classification", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `gabor_filter_bank` | `remote_sensing` | `filters` | `s$gabor_filter_bank(...)` | `wbw_run_tool("gabor_filter_bank", args = list(...), session = s)` | `Raster` | Raster output |
| `gamma_correction` | `remote_sensing` | `enhancement_contrast` | `s$gamma_correction(...)` | `wbw_run_tool("gamma_correction", args = list(...), session = s)` | `Raster` | Raster output |
| `gamma_map_filter` | `remote_sensing` | `sar` | `s$gamma_map_filter(...)` | `wbw_run_tool("gamma_map_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `gaussian_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `s$gaussian_contrast_stretch(...)` | `wbw_run_tool("gaussian_contrast_stretch", args = list(...), session = s)` | `Raster` | Raster output |
| `gaussian_curvature` | `terrain` | `derivatives` | `s$gaussian_curvature(...)` | `wbw_run_tool("gaussian_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `gaussian_filter` | `remote_sensing` | `filters` | `s$gaussian_filter(...)` | `wbw_run_tool("gaussian_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `generalize_classified_raster` | `remote_sensing` | `classification` | `s$generalize_classified_raster(...)` | `wbw_run_tool("generalize_classified_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `generalize_with_similarity` | `remote_sensing` | `classification` | `s$generalize_with_similarity(...)` | `wbw_run_tool("generalize_with_similarity", args = list(...), session = s)` | `Raster` | Raster output |
| `generate_network_nodes` | `vector` | `network_analysis` | `s$generate_network_nodes(...)` | `wbw_run_tool("generate_network_nodes", args = list(...), session = s)` | `Any` | See tool docs |
| `generating_function` | `terrain` | `derivatives` | `s$generating_function(...)` | `wbw_run_tool("generating_function", args = list(...), session = s)` | `Raster` | Raster output |
| `geomorphons` | `terrain` | `landform_indices` | `s$geomorphons(...)` | `wbw_run_tool("geomorphons", args = list(...), session = s)` | `Raster` | Raster output |
| `georeference_raster_from_control_points` | `projection_georeferencing` | `general` | `s$georeference_raster_from_control_points(...)` | `wbw_run_tool("georeference_raster_from_control_points", args = list(...), session = s)` | `Any` | See tool docs |
| `glcm_texture` | `remote_sensing` | `filters` | `s$glcm_texture(...)` | `wbw_run_tool("glcm_texture", args = list(...), session = s)` | `Raster` | Raster output |
| `greater_than` | `raster` | `general` | `s$greater_than(...)` | `wbw_run_tool("greater_than", args = list(...), session = s)` | `Raster` | Raster output |
| `guided_filter` | `remote_sensing` | `filters` | `s$guided_filter(...)` | `wbw_run_tool("guided_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `guided_uav_image_intake_workflow` | `remote_sensing` | `workflow_products` | `s$guided_uav_image_intake_workflow(...)` | `wbw_run_tool("guided_uav_image_intake_workflow", args = list(...), session = s)` | `Any` | See tool docs |
| `h_alpha_wisart_classification` | `remote_sensing` | `sar` | `s$h_alpha_wisart_classification(...)` | `wbw_run_tool("h_alpha_wisart_classification", args = list(...), session = s)` | `Raster` | Raster output |
| `hack_stream_order` | `streams` | `ordering_metrics` | `s$hack_stream_order(...)` | `wbw_run_tool("hack_stream_order", args = list(...), session = s)` | `Raster` | Raster output |
| `heat_map` | `raster` | `general` | `s$heat_map(...)` | `wbw_run_tool("heat_map", args = list(...), session = s)` | `Raster` | Raster output |
| `height_above_ground` | `lidar` | `filtering_classification` | `s$height_above_ground(...)` | `wbw_run_tool("height_above_ground", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `hexagonal_grid_from_raster_base` | `vector` | `sampling_gridding` | `s$hexagonal_grid_from_raster_base(...)` | `wbw_run_tool("hexagonal_grid_from_raster_base", args = list(...), session = s)` | `Vector` | Vector output |
| `hexagonal_grid_from_vector_base` | `vector` | `sampling_gridding` | `s$hexagonal_grid_from_vector_base(...)` | `wbw_run_tool("hexagonal_grid_from_vector_base", args = list(...), session = s)` | `Vector` | Vector output |
| `high_pass_bilateral_filter` | `remote_sensing` | `filters` | `s$high_pass_bilateral_filter(...)` | `wbw_run_tool("high_pass_bilateral_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `high_pass_filter` | `remote_sensing` | `filters` | `s$high_pass_filter(...)` | `wbw_run_tool("high_pass_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `high_pass_median_filter` | `remote_sensing` | `filters` | `s$high_pass_median_filter(...)` | `wbw_run_tool("high_pass_median_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `highest_position` | `raster` | `overlay_math` | `s$highest_position(...)` | `wbw_run_tool("highest_position", args = list(...), session = s)` | `Raster` | Raster output |
| `hillshade` | `terrain` | `general` | `s$hillshade(...)` | `wbw_run_tool("hillshade", args = list(...), session = s)` | `Raster` | Raster output |
| `hillslopes` | `hydrology` | `watersheds_basins` | `s$hillslopes(...)` | `wbw_run_tool("hillslopes", args = list(...), session = s)` | `Raster` | Raster output |
| `histogram_equalization` | `remote_sensing` | `enhancement_contrast` | `s$histogram_equalization(...)` | `wbw_run_tool("histogram_equalization", args = list(...), session = s)` | `Raster` | Raster output |
| `histogram_matching` | `remote_sensing` | `enhancement_contrast` | `s$histogram_matching(...)` | `wbw_run_tool("histogram_matching", args = list(...), session = s)` | `Raster` | Raster output |
| `histogram_matching_two_images` | `remote_sensing` | `enhancement_contrast` | `s$histogram_matching_two_images(...)` | `wbw_run_tool("histogram_matching_two_images", args = list(...), session = s)` | `Raster` | Raster output |
| `hole_proportion` | `vector` | `shape_metrics` | `s$hole_proportion(...)` | `wbw_run_tool("hole_proportion", args = list(...), session = s)` | `Vector` | Vector output |
| `horizon_angle` | `terrain` | `visibility` | `s$horizon_angle(...)` | `wbw_run_tool("horizon_angle", args = list(...), session = s)` | `Raster` | Raster output |
| `horizon_area` | `terrain` | `visibility` | `s$horizon_area(...)` | `wbw_run_tool("horizon_area", args = list(...), session = s)` | `Raster` | Raster output |
| `horizontal_excess_curvature` | `terrain` | `derivatives` | `s$horizontal_excess_curvature(...)` | `wbw_run_tool("horizontal_excess_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `horton_ratios` | `streams` | `ordering_metrics` | `s$horton_ratios(...)` | `wbw_run_tool("horton_ratios", args = list(...), session = s)` | `tuple[float, float, float, float, str | None]` | Multiple outputs (tuple) |
| `horton_stream_order` | `streams` | `ordering_metrics` | `s$horton_stream_order(...)` | `wbw_run_tool("horton_stream_order", args = list(...), session = s)` | `Raster` | Raster output |
| `hydrologic_connectivity` | `hydrology` | `hydrologic_indices` | `s$hydrologic_connectivity(...)` | `wbw_run_tool("hydrologic_connectivity", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `hypsometric_analysis` | `terrain` | `landform_indices` | `s$hypsometric_analysis(...)` | `wbw_run_tool("hypsometric_analysis", args = list(...), session = s)` | `str` | Report/path string output |
| `hypsometrically_tinted_hillshade` | `terrain` | `general` | `s$hypsometrically_tinted_hillshade(...)` | `wbw_run_tool("hypsometrically_tinted_hillshade", args = list(...), session = s)` | `Raster` | Raster output |
| `identity` | `vector` | `overlay_analysis` | `s$identity(...)` | `wbw_run_tool("identity", args = list(...), session = s)` | `Vector` | Vector output |
| `idw_interpolation` | `raster` | `general` | `s$idw_interpolation(...)` | `wbw_run_tool("idw_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `ihs_to_rgb` | `remote_sensing` | `enhancement_contrast` | `s$ihs_to_rgb(...)` | `wbw_run_tool("ihs_to_rgb", args = list(...), session = s)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `image_autocorrelation` | `raster` | `general` | `s$image_autocorrelation(...)` | `wbw_run_tool("image_autocorrelation", args = list(...), session = s)` | `str` | Report/path string output |
| `image_correlation` | `raster` | `general` | `s$image_correlation(...)` | `wbw_run_tool("image_correlation", args = list(...), session = s)` | `str` | Report/path string output |
| `image_correlation_neighbourhood_analysis` | `raster` | `local_neighborhood` | `s$image_correlation_neighbourhood_analysis(...)` | `wbw_run_tool("image_correlation_neighbourhood_analysis", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `image_difference_change_detection` | `remote_sensing` | `change_detection` | `s$image_difference_change_detection(...)` | `wbw_run_tool("image_difference_change_detection", args = list(...), session = s)` | `Any` | See tool docs |
| `image_regression` | `raster` | `general` | `s$image_regression(...)` | `wbw_run_tool("image_regression", args = list(...), session = s)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `image_segmentation` | `remote_sensing` | `obia` | `s$image_segmentation(...)` | `wbw_run_tool("image_segmentation", args = list(...), session = s)` | `Raster` | Raster output |
| `image_slider` | `remote_sensing` | `obia` | `s$image_slider(...)` | `wbw_run_tool("image_slider", args = list(...), session = s)` | `str` | Report/path string output |
| `image_stack_profile` | `remote_sensing` | `obia` | `s$image_stack_profile(...)` | `wbw_run_tool("image_stack_profile", args = list(...), session = s)` | `Any` | See tool docs |
| `impoundment_size_index` | `hydrology` | `depressions_storage` | `s$impoundment_size_index(...)` | `wbw_run_tool("impoundment_size_index", args = list(...), session = s)` | `tuple[Raster | None, Raster | None, Raster | None, Raster | None, Raster | None]` | Multiple outputs (tuple) |
| `improved_ground_point_filter` | `lidar` | `filtering_classification` | `s$improved_ground_point_filter(...)` | `wbw_run_tool("improved_ground_point_filter", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `in_season_crop_stress_intervention_planning` | `precision_agriculture` | `general` | `s$in_season_crop_stress_intervention_planning(...)` | `wbw_run_tool("in_season_crop_stress_intervention_planning", args = list(...), session = s)` | `Any` | See tool docs |
| `increment` | `raster` | `general` | `s$increment(...)` | `wbw_run_tool("increment", args = list(...), session = s)` | `Raster` | Raster output |
| `individual_tree_detection` | `lidar` | `analysis_metrics` | `s$individual_tree_detection(...)` | `wbw_run_tool("individual_tree_detection", args = list(...), session = s)` | `Vector` | Vector output |
| `individual_tree_segmentation` | `lidar` | `filtering_classification` | `s$individual_tree_segmentation(...)` | `wbw_run_tool("individual_tree_segmentation", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `inplace_add` | `raster` | `general` | `s$inplace_add(...)` | `wbw_run_tool("inplace_add", args = list(...), session = s)` | `Raster` | Raster output |
| `inplace_divide` | `raster` | `general` | `s$inplace_divide(...)` | `wbw_run_tool("inplace_divide", args = list(...), session = s)` | `Raster` | Raster output |
| `inplace_multiply` | `raster` | `general` | `s$inplace_multiply(...)` | `wbw_run_tool("inplace_multiply", args = list(...), session = s)` | `Raster` | Raster output |
| `inplace_subtract` | `raster` | `general` | `s$inplace_subtract(...)` | `wbw_run_tool("inplace_subtract", args = list(...), session = s)` | `Raster` | Raster output |
| `insert_dams` | `hydrology` | `depressions_storage` | `s$insert_dams(...)` | `wbw_run_tool("insert_dams", args = list(...), session = s)` | `Raster` | Raster output |
| `integer_division` | `raster` | `general` | `s$integer_division(...)` | `wbw_run_tool("integer_division", args = list(...), session = s)` | `Raster` | Raster output |
| `integral_image_transform` | `remote_sensing` | `filters` | `s$integral_image_transform(...)` | `wbw_run_tool("integral_image_transform", args = list(...), session = s)` | `Raster` | Raster output |
| `intersect` | `vector` | `overlay_analysis` | `s$intersect(...)` | `wbw_run_tool("intersect", args = list(...), session = s)` | `Vector` | Vector output |
| `inverse_pca` | `raster` | `general` | `s$inverse_pca(...)` | `wbw_run_tool("inverse_pca", args = list(...), session = s)` | `list[Raster]` | Raster output |
| `is_nodata` | `raster` | `general` | `s$is_nodata(...)` | `wbw_run_tool("is_nodata", args = list(...), session = s)` | `Raster` | Raster output |
| `isobasins` | `hydrology` | `watersheds_basins` | `s$isobasins(...)` | `wbw_run_tool("isobasins", args = list(...), session = s)` | `Raster` | Raster output |
| `jenson_snap_pour_points` | `hydrology` | `watersheds_basins` | `s$jenson_snap_pour_points(...)` | `wbw_run_tool("jenson_snap_pour_points", args = list(...), session = s)` | `Vector` | Vector output |
| `join_tables` | `conversion` | `vector_table_io` | `s$join_tables(...)` | `wbw_run_tool("join_tables", args = list(...), session = s)` | `Vector` | Vector output |
| `k_means_clustering` | `remote_sensing` | `classification` | `s$k_means_clustering(...)` | `wbw_run_tool("k_means_clustering", args = list(...), session = s)` | `tuple[Raster, int, str | None]` | Multiple outputs (tuple) |
| `k_nearest_mean_filter` | `remote_sensing` | `filters` | `s$k_nearest_mean_filter(...)` | `wbw_run_tool("k_nearest_mean_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `k_shortest_paths_network` | `vector` | `network_analysis` | `s$k_shortest_paths_network(...)` | `wbw_run_tool("k_shortest_paths_network", args = list(...), session = s)` | `Vector` | Vector output |
| `kappa_index` | `raster` | `general` | `s$kappa_index(...)` | `wbw_run_tool("kappa_index", args = list(...), session = s)` | `str` | Report/path string output |
| `knn_classification` | `remote_sensing` | `classification` | `s$knn_classification(...)` | `wbw_run_tool("knn_classification", args = list(...), session = s)` | `Raster` | Raster output |
| `knn_regression` | `remote_sensing` | `classification` | `s$knn_regression(...)` | `wbw_run_tool("knn_regression", args = list(...), session = s)` | `Raster` | Raster output |
| `ks_normality_test` | `raster` | `general` | `s$ks_normality_test(...)` | `wbw_run_tool("ks_normality_test", args = list(...), session = s)` | `str` | Report/path string output |
| `kuan_filter` | `remote_sensing` | `sar` | `s$kuan_filter(...)` | `wbw_run_tool("kuan_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `kuwahara_filter` | `remote_sensing` | `filters` | `s$kuwahara_filter(...)` | `wbw_run_tool("kuwahara_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `land_surface_temperature_single_channel` | `remote_sensing` | `thermal_emissivity` | `s$land_surface_temperature_single_channel(...)` | `wbw_run_tool("land_surface_temperature_single_channel", args = list(...), session = s)` | `Any` | See tool docs |
| `land_surface_temperature_split_window` | `remote_sensing` | `thermal_emissivity` | `s$land_surface_temperature_split_window(...)` | `wbw_run_tool("land_surface_temperature_split_window", args = list(...), session = s)` | `Any` | See tool docs |
| `landslide_susceptibility_assessment` | `terrain` | `workflow_products` | `s$landslide_susceptibility_assessment(...)` | `wbw_run_tool("landslide_susceptibility_assessment", args = list(...), session = s)` | `Any` | See tool docs |
| `laplacian_filter` | `remote_sensing` | `edge_feature_detection` | `s$laplacian_filter(...)` | `wbw_run_tool("laplacian_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `laplacian_of_gaussians_filter` | `remote_sensing` | `edge_feature_detection` | `s$laplacian_of_gaussians_filter(...)` | `wbw_run_tool("laplacian_of_gaussians_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `las_to_ascii` | `lidar` | `io_management` | `s$las_to_ascii(...)` | `wbw_run_tool("las_to_ascii", args = list(...), session = s)` | `str` | Report/path string output |
| `las_to_shapefile` | `lidar` | `io_management` | `s$las_to_shapefile(...)` | `wbw_run_tool("las_to_shapefile", args = list(...), session = s)` | `Vector` | Vector output |
| `layer_footprint_raster` | `vector` | `sampling_gridding` | `s$layer_footprint_raster(...)` | `wbw_run_tool("layer_footprint_raster", args = list(...), session = s)` | `Vector` | Vector output |
| `layer_footprint_vector` | `vector` | `sampling_gridding` | `s$layer_footprint_vector(...)` | `wbw_run_tool("layer_footprint_vector", args = list(...), session = s)` | `Vector` | Vector output |
| `lee_filter` | `remote_sensing` | `filters` | `s$lee_filter(...)` | `wbw_run_tool("lee_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `length_of_upstream_channels` | `streams` | `longitudinal_analysis` | `s$length_of_upstream_channels(...)` | `wbw_run_tool("length_of_upstream_channels", args = list(...), session = s)` | `Raster` | Raster output |
| `less_than` | `raster` | `general` | `s$less_than(...)` | `wbw_run_tool("less_than", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_block_maximum` | `lidar` | `interpolation_gridding` | `s$lidar_block_maximum(...)` | `wbw_run_tool("lidar_block_maximum", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_block_minimum` | `lidar` | `interpolation_gridding` | `s$lidar_block_minimum(...)` | `wbw_run_tool("lidar_block_minimum", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_change_and_disturbance_analysis` | `lidar` | `workflow_products` | `s$lidar_change_and_disturbance_analysis(...)` | `wbw_run_tool("lidar_change_and_disturbance_analysis", args = list(...), session = s)` | `Any` | See tool docs |
| `lidar_classify_subset` | `lidar` | `filtering_classification` | `s$lidar_classify_subset(...)` | `wbw_run_tool("lidar_classify_subset", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_colourize` | `lidar` | `io_management` | `s$lidar_colourize(...)` | `wbw_run_tool("lidar_colourize", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_construct_vector_tin` | `lidar` | `interpolation_gridding` | `s$lidar_construct_vector_tin(...)` | `wbw_run_tool("lidar_construct_vector_tin", args = list(...), session = s)` | `Vector` | Vector output |
| `lidar_contour` | `lidar` | `interpolation_gridding` | `s$lidar_contour(...)` | `wbw_run_tool("lidar_contour", args = list(...), session = s)` | `Vector` | Vector output |
| `lidar_digital_surface_model` | `lidar` | `interpolation_gridding` | `s$lidar_digital_surface_model(...)` | `wbw_run_tool("lidar_digital_surface_model", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_eigenvalue_features` | `lidar` | `analysis_metrics` | `s$lidar_eigenvalue_features(...)` | `wbw_run_tool("lidar_eigenvalue_features", args = list(...), session = s)` | `str` | Report/path string output |
| `lidar_elevation_slice` | `lidar` | `filtering_classification` | `s$lidar_elevation_slice(...)` | `wbw_run_tool("lidar_elevation_slice", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_ground_point_filter` | `remote_sensing` | `filters` | `s$lidar_ground_point_filter(...)` | `wbw_run_tool("lidar_ground_point_filter", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_hex_bin` | `lidar` | `interpolation_gridding` | `s$lidar_hex_bin(...)` | `wbw_run_tool("lidar_hex_bin", args = list(...), session = s)` | `Vector` | Vector output |
| `lidar_hillshade` | `lidar` | `interpolation_gridding` | `s$lidar_hillshade(...)` | `wbw_run_tool("lidar_hillshade", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_histogram` | `lidar` | `analysis_metrics` | `s$lidar_histogram(...)` | `wbw_run_tool("lidar_histogram", args = list(...), session = s)` | `str` | Report/path string output |
| `lidar_idw_interpolation` | `lidar` | `interpolation_gridding` | `s$lidar_idw_interpolation(...)` | `wbw_run_tool("lidar_idw_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_info` | `lidar` | `analysis_metrics` | `s$lidar_info(...)` | `wbw_run_tool("lidar_info", args = list(...), session = s)` | `str` | Report/path string output |
| `lidar_join` | `lidar` | `io_management` | `s$lidar_join(...)` | `wbw_run_tool("lidar_join", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_kappa` | `lidar` | `analysis_metrics` | `s$lidar_kappa(...)` | `wbw_run_tool("lidar_kappa", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_nearest_neighbour_gridding` | `lidar` | `interpolation_gridding` | `s$lidar_nearest_neighbour_gridding(...)` | `wbw_run_tool("lidar_nearest_neighbour_gridding", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_point_density` | `lidar` | `analysis_metrics` | `s$lidar_point_density(...)` | `wbw_run_tool("lidar_point_density", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_point_return_analysis` | `lidar` | `analysis_metrics` | `s$lidar_point_return_analysis(...)` | `wbw_run_tool("lidar_point_return_analysis", args = list(...), session = s)` | `str` | Report/path string output |
| `lidar_point_stats` | `lidar` | `analysis_metrics` | `s$lidar_point_stats(...)` | `wbw_run_tool("lidar_point_stats", args = list(...), session = s)` | `str` | Report/path string output |
| `lidar_qa_and_confidence` | `lidar` | `workflow_products` | `s$lidar_qa_and_confidence(...)` | `wbw_run_tool("lidar_qa_and_confidence", args = list(...), session = s)` | `Any` | See tool docs |
| `lidar_radial_basis_function_interpolation` | `lidar` | `interpolation_gridding` | `s$lidar_radial_basis_function_interpolation(...)` | `wbw_run_tool("lidar_radial_basis_function_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_ransac_planes` | `lidar` | `analysis_metrics` | `s$lidar_ransac_planes(...)` | `wbw_run_tool("lidar_ransac_planes", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_remove_outliers` | `lidar` | `filtering_classification` | `s$lidar_remove_outliers(...)` | `wbw_run_tool("lidar_remove_outliers", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_rooftop_analysis` | `lidar` | `analysis_metrics` | `s$lidar_rooftop_analysis(...)` | `wbw_run_tool("lidar_rooftop_analysis", args = list(...), session = s)` | `Vector` | Vector output |
| `lidar_segmentation` | `lidar` | `filtering_classification` | `s$lidar_segmentation(...)` | `wbw_run_tool("lidar_segmentation", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_segmentation_based_filter` | `lidar` | `filtering_classification` | `s$lidar_segmentation_based_filter(...)` | `wbw_run_tool("lidar_segmentation_based_filter", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_shift` | `lidar` | `io_management` | `s$lidar_shift(...)` | `wbw_run_tool("lidar_shift", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_sibson_interpolation` | `lidar` | `interpolation_gridding` | `s$lidar_sibson_interpolation(...)` | `wbw_run_tool("lidar_sibson_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_terrain_product_suite` | `lidar` | `workflow_products` | `s$lidar_terrain_product_suite(...)` | `wbw_run_tool("lidar_terrain_product_suite", args = list(...), session = s)` | `Any` | See tool docs |
| `lidar_thin` | `lidar` | `interpolation_gridding` | `s$lidar_thin(...)` | `wbw_run_tool("lidar_thin", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_thin_high_density` | `lidar` | `interpolation_gridding` | `s$lidar_thin_high_density(...)` | `wbw_run_tool("lidar_thin_high_density", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_tile` | `lidar` | `io_management` | `s$lidar_tile(...)` | `wbw_run_tool("lidar_tile", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `lidar_tile_footprint` | `lidar` | `interpolation_gridding` | `s$lidar_tile_footprint(...)` | `wbw_run_tool("lidar_tile_footprint", args = list(...), session = s)` | `Vector` | Vector output |
| `lidar_tin_gridding` | `lidar` | `interpolation_gridding` | `s$lidar_tin_gridding(...)` | `wbw_run_tool("lidar_tin_gridding", args = list(...), session = s)` | `Raster` | Raster output |
| `lidar_tophat_transform` | `lidar` | `io_management` | `s$lidar_tophat_transform(...)` | `wbw_run_tool("lidar_tophat_transform", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `line_detection_filter` | `remote_sensing` | `filters` | `s$line_detection_filter(...)` | `wbw_run_tool("line_detection_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `line_intersections` | `vector` | `overlay_analysis` | `s$line_intersections(...)` | `wbw_run_tool("line_intersections", args = list(...), session = s)` | `Vector` | Vector output |
| `line_polygon_clip` | `vector` | `overlay_analysis` | `s$line_polygon_clip(...)` | `wbw_run_tool("line_polygon_clip", args = list(...), session = s)` | `Vector` | Vector output |
| `line_thinning` | `remote_sensing` | `filters` | `s$line_thinning(...)` | `wbw_run_tool("line_thinning", args = list(...), session = s)` | `Raster` | Raster output |
| `linear_spectral_unmixing` | `remote_sensing` | `spectral_analytics` | `s$linear_spectral_unmixing(...)` | `wbw_run_tool("linear_spectral_unmixing", args = list(...), session = s)` | `Any` | See tool docs |
| `linearity_index` | `vector` | `shape_metrics` | `s$linearity_index(...)` | `wbw_run_tool("linearity_index", args = list(...), session = s)` | `Vector` | Vector output |
| `lines_to_polygons` | `conversion` | `geometry_topology` | `s$lines_to_polygons(...)` | `wbw_run_tool("lines_to_polygons", args = list(...), session = s)` | `Vector` | Vector output |
| `list_unique_values` | `vector` | `attribute_analysis` | `s$list_unique_values(...)` | `wbw_run_tool("list_unique_values", args = list(...), session = s)` | `str` | Report/path string output |
| `list_unique_values_raster` | `raster` | `general` | `s$list_unique_values_raster(...)` | `wbw_run_tool("list_unique_values_raster", args = list(...), session = s)` | `str` | Report/path string output |
| `ln` | `raster` | `general` | `s$ln(...)` | `wbw_run_tool("ln", args = list(...), session = s)` | `Raster` | Raster output |
| `local_hypsometric_analysis` | `terrain` | `general` | `s$local_hypsometric_analysis(...)` | `wbw_run_tool("local_hypsometric_analysis", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `locate_points_along_routes` | `vector` | `linear_referencing` | `s$locate_points_along_routes(...)` | `wbw_run_tool("locate_points_along_routes", args = list(...), session = s)` | `Vector` | Vector output |
| `location_allocation_network` | `vector` | `network_analysis` | `s$location_allocation_network(...)` | `wbw_run_tool("location_allocation_network", args = list(...), session = s)` | `Vector` | Vector output |
| `log10` | `raster` | `general` | `s$log10(...)` | `wbw_run_tool("log10", args = list(...), session = s)` | `Raster` | Raster output |
| `log2` | `raster` | `general` | `s$log2(...)` | `wbw_run_tool("log2", args = list(...), session = s)` | `Raster` | Raster output |
| `logistic_regression` | `remote_sensing` | `classification` | `s$logistic_regression(...)` | `wbw_run_tool("logistic_regression", args = list(...), session = s)` | `Raster` | Raster output |
| `long_profile` | `streams` | `longitudinal_analysis` | `s$long_profile(...)` | `wbw_run_tool("long_profile", args = list(...), session = s)` | `str | None` | Report/path string output |
| `long_profile_from_points` | `streams` | `longitudinal_analysis` | `s$long_profile_from_points(...)` | `wbw_run_tool("long_profile_from_points", args = list(...), session = s)` | `str | None` | Report/path string output |
| `longest_flowpath` | `hydrology` | `watersheds_basins` | `s$longest_flowpath(...)` | `wbw_run_tool("longest_flowpath", args = list(...), session = s)` | `Vector` | Vector output |
| `low_points_on_headwater_divides` | `terrain` | `general` | `s$low_points_on_headwater_divides(...)` | `wbw_run_tool("low_points_on_headwater_divides", args = list(...), session = s)` | `Vector` | Vector output |
| `lowest_position` | `raster` | `overlay_math` | `s$lowest_position(...)` | `wbw_run_tool("lowest_position", args = list(...), session = s)` | `Raster` | Raster output |
| `majority_filter` | `remote_sensing` | `filters` | `s$majority_filter(...)` | `wbw_run_tool("majority_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `map_features` | `raster` | `general` | `s$map_features(...)` | `wbw_run_tool("map_features", args = list(...), session = s)` | `Raster` | Raster output |
| `map_matching_v1` | `vector` | `network_analysis` | `s$map_matching_v1(...)` | `wbw_run_tool("map_matching_v1", args = list(...), session = s)` | `Vector` | Vector output |
| `map_off_terrain_objects` | `terrain` | `general` | `s$map_off_terrain_objects(...)` | `wbw_run_tool("map_off_terrain_objects", args = list(...), session = s)` | `Raster` | Raster output |
| `market_access_and_site_intelligence_workflow` | `vector` | `network_analysis` | `s$market_access_and_site_intelligence_workflow(...)` | `wbw_run_tool("market_access_and_site_intelligence_workflow", args = list(...), session = s)` | `Any` | See tool docs |
| `max` | `raster` | `general` | `s$max(...)` | `wbw_run_tool("max", args = list(...), session = s)` | `Raster` | Raster output |
| `max_absolute_overlay` | `raster` | `overlay_math` | `s$max_absolute_overlay(...)` | `wbw_run_tool("max_absolute_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `max_anisotropy_dev` | `terrain` | `multiscale_signatures` | `s$max_anisotropy_dev(...)` | `wbw_run_tool("max_anisotropy_dev", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `max_anisotropy_dev_signature` | `terrain` | `multiscale_signatures` | `s$max_anisotropy_dev_signature(...)` | `wbw_run_tool("max_anisotropy_dev_signature", args = list(...), session = s)` | `str` | Report/path string output |
| `max_branch_length` | `hydrology` | `watersheds_basins` | `s$max_branch_length(...)` | `wbw_run_tool("max_branch_length", args = list(...), session = s)` | `Raster` | Raster output |
| `max_difference_from_mean` | `terrain` | `multiscale_signatures` | `s$max_difference_from_mean(...)` | `wbw_run_tool("max_difference_from_mean", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `max_downslope_elev_change` | `terrain` | `general` | `s$max_downslope_elev_change(...)` | `wbw_run_tool("max_downslope_elev_change", args = list(...), session = s)` | `Raster` | Raster output |
| `max_elev_dev_signature` | `terrain` | `multiscale_signatures` | `s$max_elev_dev_signature(...)` | `wbw_run_tool("max_elev_dev_signature", args = list(...), session = s)` | `str` | Report/path string output |
| `max_elevation_deviation` | `terrain` | `multiscale_signatures` | `s$max_elevation_deviation(...)` | `wbw_run_tool("max_elevation_deviation", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `max_overlay` | `raster` | `overlay_math` | `s$max_overlay(...)` | `wbw_run_tool("max_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `max_upslope_elev_change` | `terrain` | `general` | `s$max_upslope_elev_change(...)` | `wbw_run_tool("max_upslope_elev_change", args = list(...), session = s)` | `Raster` | Raster output |
| `max_upslope_flowpath_length` | `hydrology` | `flow_routing` | `s$max_upslope_flowpath_length(...)` | `wbw_run_tool("max_upslope_flowpath_length", args = list(...), session = s)` | `Raster` | Raster output |
| `max_upslope_value` | `hydrology` | `flow_routing` | `s$max_upslope_value(...)` | `wbw_run_tool("max_upslope_value", args = list(...), session = s)` | `Raster` | Raster output |
| `maximal_curvature` | `terrain` | `derivatives` | `s$maximal_curvature(...)` | `wbw_run_tool("maximal_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `maximum_filter` | `remote_sensing` | `filters` | `s$maximum_filter(...)` | `wbw_run_tool("maximum_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `mdinf_flow_accum` | `hydrology` | `flow_routing` | `s$mdinf_flow_accum(...)` | `wbw_run_tool("mdinf_flow_accum", args = list(...), session = s)` | `Raster` | Raster output |
| `mean_curvature` | `terrain` | `derivatives` | `s$mean_curvature(...)` | `wbw_run_tool("mean_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `mean_filter` | `remote_sensing` | `filters` | `s$mean_filter(...)` | `wbw_run_tool("mean_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `median_filter` | `remote_sensing` | `filters` | `s$median_filter(...)` | `wbw_run_tool("median_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `medoid` | `vector` | `sampling_gridding` | `s$medoid(...)` | `wbw_run_tool("medoid", args = list(...), session = s)` | `Vector` | Vector output |
| `merge_line_segments` | `vector` | `geometry_processing` | `s$merge_line_segments(...)` | `wbw_run_tool("merge_line_segments", args = list(...), session = s)` | `Vector` | Vector output |
| `merge_table_with_csv` | `conversion` | `vector_table_io` | `s$merge_table_with_csv(...)` | `wbw_run_tool("merge_table_with_csv", args = list(...), session = s)` | `Vector` | Vector output |
| `merge_vectors` | `conversion` | `vector_table_io` | `s$merge_vectors(...)` | `wbw_run_tool("merge_vectors", args = list(...), session = s)` | `Vector` | Vector output |
| `min` | `raster` | `general` | `s$min(...)` | `wbw_run_tool("min", args = list(...), session = s)` | `Raster` | Raster output |
| `min_absolute_overlay` | `raster` | `overlay_math` | `s$min_absolute_overlay(...)` | `wbw_run_tool("min_absolute_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `min_dist_classification` | `remote_sensing` | `classification` | `s$min_dist_classification(...)` | `wbw_run_tool("min_dist_classification", args = list(...), session = s)` | `Raster` | Raster output |
| `min_downslope_elev_change` | `terrain` | `general` | `s$min_downslope_elev_change(...)` | `wbw_run_tool("min_downslope_elev_change", args = list(...), session = s)` | `Raster` | Raster output |
| `min_max_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `s$min_max_contrast_stretch(...)` | `wbw_run_tool("min_max_contrast_stretch", args = list(...), session = s)` | `Raster` | Raster output |
| `min_overlay` | `raster` | `overlay_math` | `s$min_overlay(...)` | `wbw_run_tool("min_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `mine_site_reclamation_compliance_tracker` | `terrain` | `workflow_products` | `s$mine_site_reclamation_compliance_tracker(...)` | `wbw_run_tool("mine_site_reclamation_compliance_tracker", args = list(...), session = s)` | `Any` | See tool docs |
| `minimal_curvature` | `terrain` | `derivatives` | `s$minimal_curvature(...)` | `wbw_run_tool("minimal_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `minimal_dispersion_flow_algorithm` | `hydrology` | `flow_routing` | `s$minimal_dispersion_flow_algorithm(...)` | `wbw_run_tool("minimal_dispersion_flow_algorithm", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `minimum_bounding_box` | `vector` | `geometry_processing` | `s$minimum_bounding_box(...)` | `wbw_run_tool("minimum_bounding_box", args = list(...), session = s)` | `Vector` | Vector output |
| `minimum_bounding_circle` | `vector` | `geometry_processing` | `s$minimum_bounding_circle(...)` | `wbw_run_tool("minimum_bounding_circle", args = list(...), session = s)` | `Vector` | Vector output |
| `minimum_bounding_envelope` | `vector` | `geometry_processing` | `s$minimum_bounding_envelope(...)` | `wbw_run_tool("minimum_bounding_envelope", args = list(...), session = s)` | `Vector` | Vector output |
| `minimum_convex_hull` | `vector` | `geometry_processing` | `s$minimum_convex_hull(...)` | `wbw_run_tool("minimum_convex_hull", args = list(...), session = s)` | `Vector` | Vector output |
| `minimum_filter` | `remote_sensing` | `filters` | `s$minimum_filter(...)` | `wbw_run_tool("minimum_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `minimum_noise_fraction` | `remote_sensing` | `spectral_analytics` | `s$minimum_noise_fraction(...)` | `wbw_run_tool("minimum_noise_fraction", args = list(...), session = s)` | `Any` | See tool docs |
| `modified_k_means_clustering` | `remote_sensing` | `classification` | `s$modified_k_means_clustering(...)` | `wbw_run_tool("modified_k_means_clustering", args = list(...), session = s)` | `tuple[Raster, int, str | None]` | Multiple outputs (tuple) |
| `modified_shepard_interpolation` | `raster` | `general` | `s$modified_shepard_interpolation(...)` | `wbw_run_tool("modified_shepard_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `modify_lidar` | `lidar` | `filtering_classification` | `s$modify_lidar(...)` | `wbw_run_tool("modify_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `modify_nodata_value` | `conversion` | `raster_vector_conversion` | `s$modify_nodata_value(...)` | `wbw_run_tool("modify_nodata_value", args = list(...), session = s)` | `Raster` | Raster output |
| `modulo` | `raster` | `overlay_math` | `s$modulo(...)` | `wbw_run_tool("modulo", args = list(...), session = s)` | `Raster` | Raster output |
| `mosaic` | `remote_sensing` | `enhancement_contrast` | `s$mosaic(...)` | `wbw_run_tool("mosaic", args = list(...), session = s)` | `Raster` | Raster output |
| `mosaic_with_feathering` | `remote_sensing` | `enhancement_contrast` | `s$mosaic_with_feathering(...)` | `wbw_run_tool("mosaic_with_feathering", args = list(...), session = s)` | `Raster` | Raster output |
| `multi_sensor_fusion_monitoring` | `remote_sensing` | `workflow_products` | `s$multi_sensor_fusion_monitoring(...)` | `wbw_run_tool("multi_sensor_fusion_monitoring", args = list(...), session = s)` | `Any` | See tool docs |
| `multidirectional_hillshade` | `terrain` | `general` | `s$multidirectional_hillshade(...)` | `wbw_run_tool("multidirectional_hillshade", args = list(...), session = s)` | `Raster` | Raster output |
| `multimodal_od_cost_matrix` | `vector` | `network_analysis` | `s$multimodal_od_cost_matrix(...)` | `wbw_run_tool("multimodal_od_cost_matrix", args = list(...), session = s)` | `Any` | See tool docs |
| `multimodal_routes_from_od` | `vector` | `network_analysis` | `s$multimodal_routes_from_od(...)` | `wbw_run_tool("multimodal_routes_from_od", args = list(...), session = s)` | `Any` | See tool docs |
| `multimodal_shortest_path` | `vector` | `network_analysis` | `s$multimodal_shortest_path(...)` | `wbw_run_tool("multimodal_shortest_path", args = list(...), session = s)` | `Any` | See tool docs |
| `multipart_to_singlepart` | `conversion` | `geometry_topology` | `s$multipart_to_singlepart(...)` | `wbw_run_tool("multipart_to_singlepart", args = list(...), session = s)` | `Vector` | Vector output |
| `multiply` | `raster` | `overlay_math` | `s$multiply(...)` | `wbw_run_tool("multiply", args = list(...), session = s)` | `Raster` | Raster output |
| `multiply_overlay` | `raster` | `overlay_math` | `s$multiply_overlay(...)` | `wbw_run_tool("multiply_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `multiscale_curvatures` | `terrain` | `multiscale_signatures` | `s$multiscale_curvatures(...)` | `wbw_run_tool("multiscale_curvatures", args = list(...), session = s)` | `Raster` | Raster output |
| `multiscale_elevated_index` | `terrain` | `multiscale_signatures` | `s$multiscale_elevated_index(...)` | `wbw_run_tool("multiscale_elevated_index", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_elevation_percentile` | `terrain` | `multiscale_signatures` | `s$multiscale_elevation_percentile(...)` | `wbw_run_tool("multiscale_elevation_percentile", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_low_lying_index` | `terrain` | `multiscale_signatures` | `s$multiscale_low_lying_index(...)` | `wbw_run_tool("multiscale_low_lying_index", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_roughness` | `terrain` | `multiscale_signatures` | `s$multiscale_roughness(...)` | `wbw_run_tool("multiscale_roughness", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_roughness_signature` | `terrain` | `multiscale_signatures` | `s$multiscale_roughness_signature(...)` | `wbw_run_tool("multiscale_roughness_signature", args = list(...), session = s)` | `str` | Report/path string output |
| `multiscale_std_dev_normals` | `terrain` | `multiscale_signatures` | `s$multiscale_std_dev_normals(...)` | `wbw_run_tool("multiscale_std_dev_normals", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_std_dev_normals_signature` | `terrain` | `multiscale_signatures` | `s$multiscale_std_dev_normals_signature(...)` | `wbw_run_tool("multiscale_std_dev_normals_signature", args = list(...), session = s)` | `str` | Report/path string output |
| `multiscale_topographic_position_class` | `terrain` | `landform_indices` | `s$multiscale_topographic_position_class(...)` | `wbw_run_tool("multiscale_topographic_position_class", args = list(...), session = s)` | `Raster` | Raster output |
| `multiscale_topographic_position_image` | `terrain` | `multiscale_signatures` | `s$multiscale_topographic_position_image(...)` | `wbw_run_tool("multiscale_topographic_position_image", args = list(...), session = s)` | `Raster` | Raster output |
| `narrowness_index` | `raster` | `general` | `s$narrowness_index(...)` | `wbw_run_tool("narrowness_index", args = list(...), session = s)` | `Raster` | Raster output |
| `narrowness_index_vector` | `vector` | `shape_metrics` | `s$narrowness_index_vector(...)` | `wbw_run_tool("narrowness_index_vector", args = list(...), session = s)` | `Vector` | Vector output |
| `natural_neighbour_interpolation` | `raster` | `local_neighborhood` | `s$natural_neighbour_interpolation(...)` | `wbw_run_tool("natural_neighbour_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `ndvi_based_emissivity` | `remote_sensing` | `thermal_emissivity` | `s$ndvi_based_emissivity(...)` | `wbw_run_tool("ndvi_based_emissivity", args = list(...), session = s)` | `Any` | See tool docs |
| `near` | `vector` | `overlay_analysis` | `s$near(...)` | `wbw_run_tool("near", args = list(...), session = s)` | `Vector` | Vector output |
| `nearest_neighbour_interpolation` | `raster` | `local_neighborhood` | `s$nearest_neighbour_interpolation(...)` | `wbw_run_tool("nearest_neighbour_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `negate` | `raster` | `general` | `s$negate(...)` | `wbw_run_tool("negate", args = list(...), session = s)` | `Raster` | Raster output |
| `network_accessibility_metrics` | `vector` | `network_analysis` | `s$network_accessibility_metrics(...)` | `wbw_run_tool("network_accessibility_metrics", args = list(...), session = s)` | `Any` | See tool docs |
| `network_centrality_metrics` | `vector` | `network_analysis` | `s$network_centrality_metrics(...)` | `wbw_run_tool("network_centrality_metrics", args = list(...), session = s)` | `Any` | See tool docs |
| `network_connected_components` | `vector` | `network_analysis` | `s$network_connected_components(...)` | `wbw_run_tool("network_connected_components", args = list(...), session = s)` | `Vector` | Vector output |
| `network_node_degree` | `vector` | `network_analysis` | `s$network_node_degree(...)` | `wbw_run_tool("network_node_degree", args = list(...), session = s)` | `Vector` | Vector output |
| `network_od_cost_matrix` | `vector` | `network_analysis` | `s$network_od_cost_matrix(...)` | `wbw_run_tool("network_od_cost_matrix", args = list(...), session = s)` | `str` | Report/path string output |
| `network_readiness_and_diagnostics_intelligence` | `vector` | `network_analysis` | `s$network_readiness_and_diagnostics_intelligence(...)` | `wbw_run_tool("network_readiness_and_diagnostics_intelligence", args = list(...), session = s)` | `Any` | See tool docs |
| `network_routes_from_od` | `vector` | `network_analysis` | `s$network_routes_from_od(...)` | `wbw_run_tool("network_routes_from_od", args = list(...), session = s)` | `Vector` | Vector output |
| `network_service_area` | `vector` | `network_analysis` | `s$network_service_area(...)` | `wbw_run_tool("network_service_area", args = list(...), session = s)` | `Vector` | Vector output |
| `network_topology_audit` | `vector` | `network_analysis` | `s$network_topology_audit(...)` | `wbw_run_tool("network_topology_audit", args = list(...), session = s)` | `Vector` | Vector output |
| `new_raster_from_base_raster` | `conversion` | `raster_vector_conversion` | `s$new_raster_from_base_raster(...)` | `wbw_run_tool("new_raster_from_base_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `new_raster_from_base_vector` | `conversion` | `raster_vector_conversion` | `s$new_raster_from_base_vector(...)` | `wbw_run_tool("new_raster_from_base_vector", args = list(...), session = s)` | `Raster` | Raster output |
| `nibble` | `raster` | `general` | `s$nibble(...)` | `wbw_run_tool("nibble", args = list(...), session = s)` | `Raster` | Raster output |
| `nnd_classification` | `remote_sensing` | `classification` | `s$nnd_classification(...)` | `wbw_run_tool("nnd_classification", args = list(...), session = s)` | `Raster` | Raster output |
| `non_local_means_filter` | `remote_sensing` | `filters` | `s$non_local_means_filter(...)` | `wbw_run_tool("non_local_means_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `normal_vectors` | `lidar` | `analysis_metrics` | `s$normal_vectors(...)` | `wbw_run_tool("normal_vectors", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `normalize_lidar` | `lidar` | `filtering_classification` | `s$normalize_lidar(...)` | `wbw_run_tool("normalize_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `normalized_difference_index` | `remote_sensing` | `enhancement_contrast` | `s$normalized_difference_index(...)` | `wbw_run_tool("normalized_difference_index", args = list(...), session = s)` | `Raster` | Raster output |
| `not_equal_to` | `raster` | `general` | `s$not_equal_to(...)` | `wbw_run_tool("not_equal_to", args = list(...), session = s)` | `Raster` | Raster output |
| `num_downslope_neighbours` | `terrain` | `general` | `s$num_downslope_neighbours(...)` | `wbw_run_tool("num_downslope_neighbours", args = list(...), session = s)` | `Raster` | Raster output |
| `num_inflowing_neighbours` | `hydrology` | `flow_routing` | `s$num_inflowing_neighbours(...)` | `wbw_run_tool("num_inflowing_neighbours", args = list(...), session = s)` | `Raster` | Raster output |
| `num_upslope_neighbours` | `terrain` | `general` | `s$num_upslope_neighbours(...)` | `wbw_run_tool("num_upslope_neighbours", args = list(...), session = s)` | `Raster` | Raster output |
| `obia_audit_report_pro` | `remote_sensing` | `obia` | `s$obia_audit_report_pro(...)` | `wbw_run_tool("obia_audit_report_pro", args = list(...), session = s)` | `str` | Report/path string output |
| `obia_batch_orchestrator_pro` | `remote_sensing` | `obia` | `s$obia_batch_orchestrator_pro(...)` | `wbw_run_tool("obia_batch_orchestrator_pro", args = list(...), session = s)` | `dict[str, Any]` | Report/path string output |
| `obia_pipeline_basic` | `remote_sensing` | `obia` | `s$obia_pipeline_basic(...)` | `wbw_run_tool("obia_pipeline_basic", args = list(...), session = s)` | `dict[str, Any]` | Report/path string output |
| `object_class_probability_maps` | `remote_sensing` | `obia` | `s$object_class_probability_maps(...)` | `wbw_run_tool("object_class_probability_maps", args = list(...), session = s)` | `str` | Report/path string output |
| `object_features_context_neighbors` | `remote_sensing` | `obia` | `s$object_features_context_neighbors(...)` | `wbw_run_tool("object_features_context_neighbors", args = list(...), session = s)` | `str` | Report/path string output |
| `object_features_shape_basic` | `remote_sensing` | `obia` | `s$object_features_shape_basic(...)` | `wbw_run_tool("object_features_shape_basic", args = list(...), session = s)` | `str` | Report/path string output |
| `object_features_spectral_basic` | `remote_sensing` | `obia` | `s$object_features_spectral_basic(...)` | `wbw_run_tool("object_features_spectral_basic", args = list(...), session = s)` | `str` | Report/path string output |
| `object_features_texture_glcm_basic` | `remote_sensing` | `obia` | `s$object_features_texture_glcm_basic(...)` | `wbw_run_tool("object_features_texture_glcm_basic", args = list(...), session = s)` | `str` | Report/path string output |
| `object_features_topology_relations` | `remote_sensing` | `obia` | `s$object_features_topology_relations(...)` | `wbw_run_tool("object_features_topology_relations", args = list(...), session = s)` | `str` | Report/path string output |
| `object_uncertainty_diagnostics_pro` | `remote_sensing` | `obia` | `s$object_uncertainty_diagnostics_pro(...)` | `wbw_run_tool("object_uncertainty_diagnostics_pro", args = list(...), session = s)` | `str` | Report/path string output |
| `objects_boundary_refinement_pro` | `remote_sensing` | `obia` | `s$objects_boundary_refinement_pro(...)` | `wbw_run_tool("objects_boundary_refinement_pro", args = list(...), session = s)` | `Raster` | Raster output |
| `objects_enforce_min_mapping_unit` | `remote_sensing` | `obia` | `s$objects_enforce_min_mapping_unit(...)` | `wbw_run_tool("objects_enforce_min_mapping_unit", args = list(...), session = s)` | `Raster` | Raster output |
| `od_sensitivity_analysis` | `vector` | `network_analysis` | `s$od_sensitivity_analysis(...)` | `wbw_run_tool("od_sensitivity_analysis", args = list(...), session = s)` | `Any` | See tool docs |
| `olympic_filter` | `remote_sensing` | `filters` | `s$olympic_filter(...)` | `wbw_run_tool("olympic_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `opening` | `remote_sensing` | `filters` | `s$opening(...)` | `wbw_run_tool("opening", args = list(...), session = s)` | `Raster` | Raster output |
| `openness` | `terrain` | `visibility` | `s$openness(...)` | `wbw_run_tool("openness", args = list(...), session = s)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `orthorectification` | `projection_georeferencing` | `general` | `s$orthorectification(...)` | `wbw_run_tool("orthorectification", args = list(...), session = s)` | `Any` | See tool docs |
| `otsu_thresholding` | `remote_sensing` | `classification` | `s$otsu_thresholding(...)` | `wbw_run_tool("otsu_thresholding", args = list(...), session = s)` | `Raster` | Raster output |
| `paired_sample_t_test` | `raster` | `general` | `s$paired_sample_t_test(...)` | `wbw_run_tool("paired_sample_t_test", args = list(...), session = s)` | `str` | Report/path string output |
| `panchromatic_sharpening` | `remote_sensing` | `enhancement_contrast` | `s$panchromatic_sharpening(...)` | `wbw_run_tool("panchromatic_sharpening", args = list(...), session = s)` | `Raster` | Raster output |
| `parallelepiped_classification` | `remote_sensing` | `classification` | `s$parallelepiped_classification(...)` | `wbw_run_tool("parallelepiped_classification", args = list(...), session = s)` | `Raster` | Raster output |
| `parcel_and_land_fabric_topology_compliance_workflow` | `vector` | `workflow_products` | `s$parcel_and_land_fabric_topology_compliance_workflow(...)` | `wbw_run_tool("parcel_and_land_fabric_topology_compliance_workflow", args = list(...), session = s)` | `Any` | See tool docs |
| `patch_orientation` | `vector` | `shape_metrics` | `s$patch_orientation(...)` | `wbw_run_tool("patch_orientation", args = list(...), session = s)` | `Vector` | Vector output |
| `pca_based_change_detection` | `remote_sensing` | `change_detection` | `s$pca_based_change_detection(...)` | `wbw_run_tool("pca_based_change_detection", args = list(...), session = s)` | `Any` | See tool docs |
| `pennock_landform_classification` | `terrain` | `landform_indices` | `s$pennock_landform_classification(...)` | `wbw_run_tool("pennock_landform_classification", args = list(...), session = s)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `percent_elev_range` | `terrain` | `landform_indices` | `s$percent_elev_range(...)` | `wbw_run_tool("percent_elev_range", args = list(...), session = s)` | `Raster` | Raster output |
| `percent_equal_to` | `raster` | `overlay_math` | `s$percent_equal_to(...)` | `wbw_run_tool("percent_equal_to", args = list(...), session = s)` | `Raster` | Raster output |
| `percent_greater_than` | `raster` | `overlay_math` | `s$percent_greater_than(...)` | `wbw_run_tool("percent_greater_than", args = list(...), session = s)` | `Raster` | Raster output |
| `percent_less_than` | `raster` | `overlay_math` | `s$percent_less_than(...)` | `wbw_run_tool("percent_less_than", args = list(...), session = s)` | `Raster` | Raster output |
| `percentage_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `s$percentage_contrast_stretch(...)` | `wbw_run_tool("percentage_contrast_stretch", args = list(...), session = s)` | `Raster` | Raster output |
| `percentile_filter` | `remote_sensing` | `filters` | `s$percentile_filter(...)` | `wbw_run_tool("percentile_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `perimeter_area_ratio` | `vector` | `shape_metrics` | `s$perimeter_area_ratio(...)` | `wbw_run_tool("perimeter_area_ratio", args = list(...), session = s)` | `Vector` | Vector output |
| `phi_coefficient` | `raster` | `general` | `s$phi_coefficient(...)` | `wbw_run_tool("phi_coefficient", args = list(...), session = s)` | `str` | Report/path string output |
| `pick_from_list` | `raster` | `overlay_math` | `s$pick_from_list(...)` | `wbw_run_tool("pick_from_list", args = list(...), session = s)` | `Raster` | Raster output |
| `piecewise_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `s$piecewise_contrast_stretch(...)` | `wbw_run_tool("piecewise_contrast_stretch", args = list(...), session = s)` | `Raster` | Raster output |
| `plan_curvature` | `terrain` | `derivatives` | `s$plan_curvature(...)` | `wbw_run_tool("plan_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `points_along_lines` | `vector` | `linear_referencing` | `s$points_along_lines(...)` | `wbw_run_tool("points_along_lines", args = list(...), session = s)` | `Vector` | Vector output |
| `polygon_area` | `vector` | `shape_metrics` | `s$polygon_area(...)` | `wbw_run_tool("polygon_area", args = list(...), session = s)` | `Vector` | Vector output |
| `polygon_long_axis` | `vector` | `shape_metrics` | `s$polygon_long_axis(...)` | `wbw_run_tool("polygon_long_axis", args = list(...), session = s)` | `Vector` | Vector output |
| `polygon_perimeter` | `vector` | `shape_metrics` | `s$polygon_perimeter(...)` | `wbw_run_tool("polygon_perimeter", args = list(...), session = s)` | `Vector` | Vector output |
| `polygon_short_axis` | `vector` | `shape_metrics` | `s$polygon_short_axis(...)` | `wbw_run_tool("polygon_short_axis", args = list(...), session = s)` | `Vector` | Vector output |
| `polygonize` | `vector` | `geometry_processing` | `s$polygonize(...)` | `wbw_run_tool("polygonize", args = list(...), session = s)` | `Vector` | Vector output |
| `polygons_to_lines` | `conversion` | `geometry_topology` | `s$polygons_to_lines(...)` | `wbw_run_tool("polygons_to_lines", args = list(...), session = s)` | `Vector` | Vector output |
| `polygons_to_segments` | `remote_sensing` | `obia` | `s$polygons_to_segments(...)` | `wbw_run_tool("polygons_to_segments", args = list(...), session = s)` | `Raster` | Raster output |
| `post_classification_change` | `remote_sensing` | `change_detection` | `s$post_classification_change(...)` | `wbw_run_tool("post_classification_change", args = list(...), session = s)` | `Any` | See tool docs |
| `power` | `raster` | `overlay_math` | `s$power(...)` | `wbw_run_tool("power", args = list(...), session = s)` | `Raster` | Raster output |
| `precision_ag_yield_zone_intelligence` | `precision_agriculture` | `general` | `s$precision_ag_yield_zone_intelligence(...)` | `wbw_run_tool("precision_ag_yield_zone_intelligence", args = list(...), session = s)` | `Any` | See tool docs |
| `precision_irrigation_optimization` | `precision_agriculture` | `general` | `s$precision_irrigation_optimization(...)` | `wbw_run_tool("precision_irrigation_optimization", args = list(...), session = s)` | `Any` | See tool docs |
| `prewitt_filter` | `remote_sensing` | `edge_feature_detection` | `s$prewitt_filter(...)` | `wbw_run_tool("prewitt_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `principal_component_analysis` | `raster` | `general` | `s$principal_component_analysis(...)` | `wbw_run_tool("principal_component_analysis", args = list(...), session = s)` | `list[Raster]` | Raster output |
| `principal_curvature_direction` | `terrain` | `derivatives` | `s$principal_curvature_direction(...)` | `wbw_run_tool("principal_curvature_direction", args = list(...), session = s)` | `Raster` | Raster output |
| `print_geotiff_tags` | `raster` | `general` | `s$print_geotiff_tags(...)` | `wbw_run_tool("print_geotiff_tags", args = list(...), session = s)` | `str` | Report/path string output |
| `profile` | `terrain` | `general` | `s$profile(...)` | `wbw_run_tool("profile", args = list(...), session = s)` | `str` | Report/path string output |
| `profile_curvature` | `terrain` | `derivatives` | `s$profile_curvature(...)` | `wbw_run_tool("profile_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `propagate_labels_across_hierarchy` | `remote_sensing` | `obia` | `s$propagate_labels_across_hierarchy(...)` | `wbw_run_tool("propagate_labels_across_hierarchy", args = list(...), session = s)` | `str` | Report/path string output |
| `prune_vector_streams` | `streams` | `network_extraction` | `s$prune_vector_streams(...)` | `wbw_run_tool("prune_vector_streams", args = list(...), session = s)` | `Vector` | Vector output |
| `qin_flow_accumulation` | `hydrology` | `flow_routing` | `s$qin_flow_accumulation(...)` | `wbw_run_tool("qin_flow_accumulation", args = list(...), session = s)` | `Raster` | Raster output |
| `quantiles` | `raster` | `general` | `s$quantiles(...)` | `wbw_run_tool("quantiles", args = list(...), session = s)` | `Raster` | Raster output |
| `quinn_flow_accumulation` | `hydrology` | `flow_routing` | `s$quinn_flow_accumulation(...)` | `wbw_run_tool("quinn_flow_accumulation", args = list(...), session = s)` | `Raster` | Raster output |
| `radial_basis_function_interpolation` | `raster` | `general` | `s$radial_basis_function_interpolation(...)` | `wbw_run_tool("radial_basis_function_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `radius_of_gyration` | `raster` | `general` | `s$radius_of_gyration(...)` | `wbw_run_tool("radius_of_gyration", args = list(...), session = s)` | `Raster` | Raster output |
| `raise_walls` | `hydrology` | `depressions_storage` | `s$raise_walls(...)` | `wbw_run_tool("raise_walls", args = list(...), session = s)` | `Raster` | Raster output |
| `random_field` | `raster` | `general` | `s$random_field(...)` | `wbw_run_tool("random_field", args = list(...), session = s)` | `Raster` | Raster output |
| `random_forest_classification` | `remote_sensing` | `classification` | `s$random_forest_classification(...)` | `wbw_run_tool("random_forest_classification", args = list(...), session = s)` | `Raster` | Raster output |
| `random_forest_classification_fit` | `raster` | `general` | `s$random_forest_classification_fit(...)` | `wbw_run_tool("random_forest_classification_fit", args = list(...), session = s)` | `list[int]` | See tool docs |
| `random_forest_classification_predict` | `raster` | `general` | `s$random_forest_classification_predict(...)` | `wbw_run_tool("random_forest_classification_predict", args = list(...), session = s)` | `Raster` | Raster output |
| `random_forest_regression` | `remote_sensing` | `classification` | `s$random_forest_regression(...)` | `wbw_run_tool("random_forest_regression", args = list(...), session = s)` | `Raster` | Raster output |
| `random_forest_regression_fit` | `raster` | `general` | `s$random_forest_regression_fit(...)` | `wbw_run_tool("random_forest_regression_fit", args = list(...), session = s)` | `list[int]` | See tool docs |
| `random_forest_regression_predict` | `raster` | `general` | `s$random_forest_regression_predict(...)` | `wbw_run_tool("random_forest_regression_predict", args = list(...), session = s)` | `Raster` | Raster output |
| `random_points_in_polygon` | `vector` | `sampling_gridding` | `s$random_points_in_polygon(...)` | `wbw_run_tool("random_points_in_polygon", args = list(...), session = s)` | `Vector` | Vector output |
| `random_sample` | `raster` | `general` | `s$random_sample(...)` | `wbw_run_tool("random_sample", args = list(...), session = s)` | `Raster` | Raster output |
| `range_filter` | `remote_sensing` | `filters` | `s$range_filter(...)` | `wbw_run_tool("range_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `raster_area` | `raster` | `general` | `s$raster_area(...)` | `wbw_run_tool("raster_area", args = list(...), session = s)` | `Raster` | Raster output |
| `raster_calculator` | `raster` | `general` | `s$raster_calculator(...)` | `wbw_run_tool("raster_calculator", args = list(...), session = s)` | `Raster` | Raster output |
| `raster_cell_assignment` | `raster` | `general` | `s$raster_cell_assignment(...)` | `wbw_run_tool("raster_cell_assignment", args = list(...), session = s)` | `Raster` | Raster output |
| `raster_histogram` | `raster` | `general` | `s$raster_histogram(...)` | `wbw_run_tool("raster_histogram", args = list(...), session = s)` | `str` | Report/path string output |
| `raster_perimeter` | `raster` | `general` | `s$raster_perimeter(...)` | `wbw_run_tool("raster_perimeter", args = list(...), session = s)` | `Raster` | Raster output |
| `raster_streams_to_vector` | `streams` | `network_extraction` | `s$raster_streams_to_vector(...)` | `wbw_run_tool("raster_streams_to_vector", args = list(...), session = s)` | `Vector` | Vector output |
| `raster_summary_stats` | `raster` | `general` | `s$raster_summary_stats(...)` | `wbw_run_tool("raster_summary_stats", args = list(...), session = s)` | `str` | Report/path string output |
| `raster_to_vector_lines` | `conversion` | `raster_vector_conversion` | `s$raster_to_vector_lines(...)` | `wbw_run_tool("raster_to_vector_lines", args = list(...), session = s)` | `Vector` | Vector output |
| `raster_to_vector_points` | `conversion` | `raster_vector_conversion` | `s$raster_to_vector_points(...)` | `wbw_run_tool("raster_to_vector_points", args = list(...), session = s)` | `Vector` | Vector output |
| `raster_to_vector_polygons` | `conversion` | `raster_vector_conversion` | `s$raster_to_vector_polygons(...)` | `wbw_run_tool("raster_to_vector_polygons", args = list(...), session = s)` | `Vector` | Vector output |
| `rasterize_streams` | `streams` | `network_extraction` | `s$rasterize_streams(...)` | `wbw_run_tool("rasterize_streams", args = list(...), session = s)` | `Raster` | Raster output |
| `reciprocal` | `raster` | `general` | `s$reciprocal(...)` | `wbw_run_tool("reciprocal", args = list(...), session = s)` | `Raster` | Raster output |
| `reclass` | `raster` | `reclass_mask` | `s$reclass(...)` | `wbw_run_tool("reclass", args = list(...), session = s)` | `Raster` | Raster output |
| `reclass_equal_interval` | `raster` | `reclass_mask` | `s$reclass_equal_interval(...)` | `wbw_run_tool("reclass_equal_interval", args = list(...), session = s)` | `Raster` | Raster output |
| `recover_flightline_info` | `lidar` | `io_management` | `s$recover_flightline_info(...)` | `wbw_run_tool("recover_flightline_info", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `rectangular_grid_from_raster_base` | `vector` | `sampling_gridding` | `s$rectangular_grid_from_raster_base(...)` | `wbw_run_tool("rectangular_grid_from_raster_base", args = list(...), session = s)` | `Vector` | Vector output |
| `rectangular_grid_from_vector_base` | `vector` | `sampling_gridding` | `s$rectangular_grid_from_vector_base(...)` | `wbw_run_tool("rectangular_grid_from_vector_base", args = list(...), session = s)` | `Vector` | Vector output |
| `refined_lee_filter` | `remote_sensing` | `sar` | `s$refined_lee_filter(...)` | `wbw_run_tool("refined_lee_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `registration_oriented_feature_workflow` | `remote_sensing` | `workflow_products` | `s$registration_oriented_feature_workflow(...)` | `wbw_run_tool("registration_oriented_feature_workflow", args = list(...), session = s)` | `Any` | See tool docs |
| `reinitialize_attribute_table` | `conversion` | `vector_table_io` | `s$reinitialize_attribute_table(...)` | `wbw_run_tool("reinitialize_attribute_table", args = list(...), session = s)` | `Vector` | Vector output |
| `related_circumscribing_circle` | `vector` | `shape_metrics` | `s$related_circumscribing_circle(...)` | `wbw_run_tool("related_circumscribing_circle", args = list(...), session = s)` | `Vector` | Vector output |
| `relative_aspect` | `terrain` | `derivatives` | `s$relative_aspect(...)` | `wbw_run_tool("relative_aspect", args = list(...), session = s)` | `Raster` | Raster output |
| `relative_stream_power_index` | `hydrology` | `hydrologic_indices` | `s$relative_stream_power_index(...)` | `wbw_run_tool("relative_stream_power_index", args = list(...), session = s)` | `Raster` | Raster output |
| `relative_topographic_position` | `terrain` | `landform_indices` | `s$relative_topographic_position(...)` | `wbw_run_tool("relative_topographic_position", args = list(...), session = s)` | `Raster` | Raster output |
| `remote_sensing_change_detection` | `remote_sensing` | `change_detection` | `s$remote_sensing_change_detection(...)` | `wbw_run_tool("remote_sensing_change_detection", args = list(...), session = s)` | `tuple[Raster, Raster, str]` | Multiple outputs (tuple) |
| `remove_duplicates` | `lidar` | `filtering_classification` | `s$remove_duplicates(...)` | `wbw_run_tool("remove_duplicates", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `remove_off_terrain_objects` | `terrain` | `general` | `s$remove_off_terrain_objects(...)` | `wbw_run_tool("remove_off_terrain_objects", args = list(...), session = s)` | `Raster` | Raster output |
| `remove_polygon_holes` | `conversion` | `geometry_topology` | `s$remove_polygon_holes(...)` | `wbw_run_tool("remove_polygon_holes", args = list(...), session = s)` | `Vector` | Vector output |
| `remove_raster_polygon_holes` | `conversion` | `raster_vector_conversion` | `s$remove_raster_polygon_holes(...)` | `wbw_run_tool("remove_raster_polygon_holes", args = list(...), session = s)` | `Raster` | Raster output |
| `remove_short_streams` | `streams` | `network_extraction` | `s$remove_short_streams(...)` | `wbw_run_tool("remove_short_streams", args = list(...), session = s)` | `Raster` | Raster output |
| `remove_spurs` | `remote_sensing` | `filters` | `s$remove_spurs(...)` | `wbw_run_tool("remove_spurs", args = list(...), session = s)` | `Raster` | Raster output |
| `rename_field` | `vector` | `attribute_analysis` | `s$rename_field(...)` | `wbw_run_tool("rename_field", args = list(...), session = s)` | `Vector` | Vector output |
| `repair_stream_vector_topology` | `streams` | `network_extraction` | `s$repair_stream_vector_topology(...)` | `wbw_run_tool("repair_stream_vector_topology", args = list(...), session = s)` | `Vector` | Vector output |
| `representative_point_vector` | `vector` | `geometry_processing` | `s$representative_point_vector(...)` | `wbw_run_tool("representative_point_vector", args = list(...), session = s)` | `Any` | See tool docs |
| `reproject_lidar` | `projection_georeferencing` | `general` | `s$reproject_lidar(...)` | `wbw_run_tool("reproject_lidar", args = list(...), session = s)` | `Any` | See tool docs |
| `reproject_raster` | `projection_georeferencing` | `general` | `s$reproject_raster(...)` | `wbw_run_tool("reproject_raster", args = list(...), session = s)` | `Any` | See tool docs |
| `reproject_vector` | `projection_georeferencing` | `general` | `s$reproject_vector(...)` | `wbw_run_tool("reproject_vector", args = list(...), session = s)` | `Any` | See tool docs |
| `resample` | `remote_sensing` | `enhancement_contrast` | `s$resample(...)` | `wbw_run_tool("resample", args = list(...), session = s)` | `Raster` | Raster output |
| `rescale_value_range` | `raster` | `general` | `s$rescale_value_range(...)` | `wbw_run_tool("rescale_value_range", args = list(...), session = s)` | `Raster` | Raster output |
| `rgb_to_ihs` | `remote_sensing` | `enhancement_contrast` | `s$rgb_to_ihs(...)` | `wbw_run_tool("rgb_to_ihs", args = list(...), session = s)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `rho8_flow_accum` | `hydrology` | `flow_routing` | `s$rho8_flow_accum(...)` | `wbw_run_tool("rho8_flow_accum", args = list(...), session = s)` | `Raster` | Raster output |
| `rho8_pointer` | `hydrology` | `flow_routing` | `s$rho8_pointer(...)` | `wbw_run_tool("rho8_pointer", args = list(...), session = s)` | `Raster` | Raster output |
| `ridge_and_valley_vectors` | `terrain` | `general` | `s$ridge_and_valley_vectors(...)` | `wbw_run_tool("ridge_and_valley_vectors", args = list(...), session = s)` | `tuple[Vector, Vector]` | Multiple outputs (tuple) |
| `ring_curvature` | `terrain` | `derivatives` | `s$ring_curvature(...)` | `wbw_run_tool("ring_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `river_centerlines` | `streams` | `network_extraction` | `s$river_centerlines(...)` | `wbw_run_tool("river_centerlines", args = list(...), session = s)` | `Vector` | Vector output |
| `river_corridor_health_assessment` | `terrain` | `workflow_products` | `s$river_corridor_health_assessment(...)` | `wbw_run_tool("river_corridor_health_assessment", args = list(...), session = s)` | `Any` | See tool docs |
| `roberts_cross_filter` | `remote_sensing` | `edge_feature_detection` | `s$roberts_cross_filter(...)` | `wbw_run_tool("roberts_cross_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `root_mean_square_error` | `raster` | `general` | `s$root_mean_square_error(...)` | `wbw_run_tool("root_mean_square_error", args = list(...), session = s)` | `str` | Report/path string output |
| `rotor` | `terrain` | `derivatives` | `s$rotor(...)` | `wbw_run_tool("rotor", args = list(...), session = s)` | `Raster` | Raster output |
| `round` | `raster` | `general` | `s$round(...)` | `wbw_run_tool("round", args = list(...), session = s)` | `Raster` | Raster output |
| `route_calibrate` | `vector` | `linear_referencing` | `s$route_calibrate(...)` | `wbw_run_tool("route_calibrate", args = list(...), session = s)` | `Any` | See tool docs |
| `route_event_governance_for_linear_assets` | `vector` | `linear_referencing` | `s$route_event_governance_for_linear_assets(...)` | `wbw_run_tool("route_event_governance_for_linear_assets", args = list(...), session = s)` | `Any` | See tool docs |
| `route_event_lines_from_layer` | `vector` | `linear_referencing` | `s$route_event_lines_from_layer(...)` | `wbw_run_tool("route_event_lines_from_layer", args = list(...), session = s)` | `Vector` | Vector output |
| `route_event_lines_from_table` | `vector` | `linear_referencing` | `s$route_event_lines_from_table(...)` | `wbw_run_tool("route_event_lines_from_table", args = list(...), session = s)` | `Vector` | Vector output |
| `route_event_merge` | `vector` | `linear_referencing` | `s$route_event_merge(...)` | `wbw_run_tool("route_event_merge", args = list(...), session = s)` | `Any` | See tool docs |
| `route_event_overlay` | `vector` | `linear_referencing` | `s$route_event_overlay(...)` | `wbw_run_tool("route_event_overlay", args = list(...), session = s)` | `Any` | See tool docs |
| `route_event_points_from_layer` | `vector` | `linear_referencing` | `s$route_event_points_from_layer(...)` | `wbw_run_tool("route_event_points_from_layer", args = list(...), session = s)` | `Vector` | Vector output |
| `route_event_points_from_table` | `vector` | `linear_referencing` | `s$route_event_points_from_table(...)` | `wbw_run_tool("route_event_points_from_table", args = list(...), session = s)` | `Vector` | Vector output |
| `route_event_split` | `vector` | `linear_referencing` | `s$route_event_split(...)` | `wbw_run_tool("route_event_split", args = list(...), session = s)` | `Any` | See tool docs |
| `route_measure_qa` | `vector` | `linear_referencing` | `s$route_measure_qa(...)` | `wbw_run_tool("route_measure_qa", args = list(...), session = s)` | `Any` | See tool docs |
| `route_recalibrate` | `vector` | `linear_referencing` | `s$route_recalibrate(...)` | `wbw_run_tool("route_recalibrate", args = list(...), session = s)` | `Any` | See tool docs |
| `ruggedness_index` | `terrain` | `roughness_texture` | `s$ruggedness_index(...)` | `wbw_run_tool("ruggedness_index", args = list(...), session = s)` | `Raster` | Raster output |
| `sar_analysis_readiness` | `remote_sensing` | `sar` | `s$sar_analysis_readiness(...)` | `wbw_run_tool("sar_analysis_readiness", args = list(...), session = s)` | `Any` | See tool docs |
| `sar_coregistration` | `remote_sensing` | `sar` | `s$sar_coregistration(...)` | `wbw_run_tool("sar_coregistration", args = list(...), session = s)` | `Any` | See tool docs |
| `sar_interferogram_coherence` | `remote_sensing` | `sar` | `s$sar_interferogram_coherence(...)` | `wbw_run_tool("sar_interferogram_coherence", args = list(...), session = s)` | `Any` | See tool docs |
| `savitzky_golay_2d_filter` | `remote_sensing` | `filters` | `s$savitzky_golay_2d_filter(...)` | `wbw_run_tool("savitzky_golay_2d_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `scharr_filter` | `remote_sensing` | `filters` | `s$scharr_filter(...)` | `wbw_run_tool("scharr_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `sediment_transport_index` | `hydrology` | `hydrologic_indices` | `s$sediment_transport_index(...)` | `wbw_run_tool("sediment_transport_index", args = list(...), session = s)` | `Raster` | Raster output |
| `segment_graph_felzenszwalb` | `remote_sensing` | `obia` | `s$segment_graph_felzenszwalb(...)` | `wbw_run_tool("segment_graph_felzenszwalb", args = list(...), session = s)` | `Raster` | Raster output |
| `segment_multiresolution_hierarchical` | `remote_sensing` | `obia` | `s$segment_multiresolution_hierarchical(...)` | `wbw_run_tool("segment_multiresolution_hierarchical", args = list(...), session = s)` | `tuple[Raster, Raster, str]` | Multiple outputs (tuple) |
| `segment_scale_parameter_optimizer` | `remote_sensing` | `obia` | `s$segment_scale_parameter_optimizer(...)` | `wbw_run_tool("segment_scale_parameter_optimizer", args = list(...), session = s)` | `str` | Report/path string output |
| `segment_slic_superpixels` | `remote_sensing` | `obia` | `s$segment_slic_superpixels(...)` | `wbw_run_tool("segment_slic_superpixels", args = list(...), session = s)` | `Raster` | Raster output |
| `segment_watershed_markers` | `remote_sensing` | `obia` | `s$segment_watershed_markers(...)` | `wbw_run_tool("segment_watershed_markers", args = list(...), session = s)` | `Raster` | Raster output |
| `segments_merge_small_regions` | `remote_sensing` | `obia` | `s$segments_merge_small_regions(...)` | `wbw_run_tool("segments_merge_small_regions", args = list(...), session = s)` | `Raster` | Raster output |
| `segments_split_low_cohesion` | `remote_sensing` | `obia` | `s$segments_split_low_cohesion(...)` | `wbw_run_tool("segments_split_low_cohesion", args = list(...), session = s)` | `Raster` | Raster output |
| `segments_to_polygons` | `remote_sensing` | `obia` | `s$segments_to_polygons(...)` | `wbw_run_tool("segments_to_polygons", args = list(...), session = s)` | `str` | Report/path string output |
| `select_by_location` | `vector` | `overlay_analysis` | `s$select_by_location(...)` | `wbw_run_tool("select_by_location", args = list(...), session = s)` | `Vector` | Vector output |
| `select_tiles_by_polygon` | `lidar` | `io_management` | `s$select_tiles_by_polygon(...)` | `wbw_run_tool("select_tiles_by_polygon", args = list(...), session = s)` | `str` | Report/path string output |
| `service_area_planning_and_coverage_optimization` | `vector` | `network_analysis` | `s$service_area_planning_and_coverage_optimization(...)` | `wbw_run_tool("service_area_planning_and_coverage_optimization", args = list(...), session = s)` | `Any` | See tool docs |
| `set_nodata_value` | `conversion` | `raster_vector_conversion` | `s$set_nodata_value(...)` | `wbw_run_tool("set_nodata_value", args = list(...), session = s)` | `Raster` | Raster output |
| `shadow_animation` | `terrain` | `visibility` | `s$shadow_animation(...)` | `wbw_run_tool("shadow_animation", args = list(...), session = s)` | `tuple[str, str]` | Multiple outputs (tuple) |
| `shadow_image` | `terrain` | `visibility` | `s$shadow_image(...)` | `wbw_run_tool("shadow_image", args = list(...), session = s)` | `Raster` | Raster output |
| `shape_complexity_index_raster` | `raster` | `general` | `s$shape_complexity_index_raster(...)` | `wbw_run_tool("shape_complexity_index_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `shape_complexity_index_vector` | `vector` | `shape_metrics` | `s$shape_complexity_index_vector(...)` | `wbw_run_tool("shape_complexity_index_vector", args = list(...), session = s)` | `Vector` | Vector output |
| `shape_index` | `terrain` | `derivatives` | `s$shape_index(...)` | `wbw_run_tool("shape_index", args = list(...), session = s)` | `Raster` | Raster output |
| `shortest_path_network` | `vector` | `network_analysis` | `s$shortest_path_network(...)` | `wbw_run_tool("shortest_path_network", args = list(...), session = s)` | `Vector` | Vector output |
| `shreve_stream_magnitude` | `streams` | `ordering_metrics` | `s$shreve_stream_magnitude(...)` | `wbw_run_tool("shreve_stream_magnitude", args = list(...), session = s)` | `Raster` | Raster output |
| `sidewalk_vegetation_accessibility_monitoring` | `lidar` | `workflow_products` | `s$sidewalk_vegetation_accessibility_monitoring(...)` | `wbw_run_tool("sidewalk_vegetation_accessibility_monitoring", args = list(...), session = s)` | `Any` | See tool docs |
| `sieve` | `raster` | `general` | `s$sieve(...)` | `wbw_run_tool("sieve", args = list(...), session = s)` | `Raster` | Raster output |
| `sigmoidal_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `s$sigmoidal_contrast_stretch(...)` | `wbw_run_tool("sigmoidal_contrast_stretch", args = list(...), session = s)` | `Raster` | Raster output |
| `simplify_features` | `vector` | `geometry_processing` | `s$simplify_features(...)` | `wbw_run_tool("simplify_features", args = list(...), session = s)` | `Vector` | Vector output |
| `sin` | `raster` | `general` | `s$sin(...)` | `wbw_run_tool("sin", args = list(...), session = s)` | `Raster` | Raster output |
| `singlepart_to_multipart` | `conversion` | `geometry_topology` | `s$singlepart_to_multipart(...)` | `wbw_run_tool("singlepart_to_multipart", args = list(...), session = s)` | `Vector` | Vector output |
| `sinh` | `raster` | `general` | `s$sinh(...)` | `wbw_run_tool("sinh", args = list(...), session = s)` | `Raster` | Raster output |
| `sink` | `hydrology` | `depressions_storage` | `s$sink(...)` | `wbw_run_tool("sink", args = list(...), session = s)` | `Raster` | Raster output |
| `sky_view_factor` | `terrain` | `visibility` | `s$sky_view_factor(...)` | `wbw_run_tool("sky_view_factor", args = list(...), session = s)` | `Raster` | Raster output |
| `skyline_analysis` | `terrain` | `visibility` | `s$skyline_analysis(...)` | `wbw_run_tool("skyline_analysis", args = list(...), session = s)` | `tuple[Vector, str]` | Multiple outputs (tuple) |
| `slope` | `terrain` | `derivatives` | `s$slope(...)` | `wbw_run_tool("slope", args = list(...), session = s)` | `Raster` | Raster output |
| `slope_vs_aspect_plot` | `terrain` | `general` | `s$slope_vs_aspect_plot(...)` | `wbw_run_tool("slope_vs_aspect_plot", args = list(...), session = s)` | `str` | Report/path string output |
| `slope_vs_elev_plot` | `terrain` | `general` | `s$slope_vs_elev_plot(...)` | `wbw_run_tool("slope_vs_elev_plot", args = list(...), session = s)` | `str` | Report/path string output |
| `smooth_vectors` | `vector` | `geometry_processing` | `s$smooth_vectors(...)` | `wbw_run_tool("smooth_vectors", args = list(...), session = s)` | `Vector` | Vector output |
| `smooth_vegetation_residual` | `terrain` | `general` | `s$smooth_vegetation_residual(...)` | `wbw_run_tool("smooth_vegetation_residual", args = list(...), session = s)` | `Raster` | Raster output |
| `snap_endnodes` | `vector` | `geometry_processing` | `s$snap_endnodes(...)` | `wbw_run_tool("snap_endnodes", args = list(...), session = s)` | `Vector` | Vector output |
| `snap_events_to_routes` | `vector` | `linear_referencing` | `s$snap_events_to_routes(...)` | `wbw_run_tool("snap_events_to_routes", args = list(...), session = s)` | `Any` | See tool docs |
| `snap_points_to_network` | `vector` | `network_analysis` | `s$snap_points_to_network(...)` | `wbw_run_tool("snap_points_to_network", args = list(...), session = s)` | `Any` | See tool docs |
| `snap_pour_points` | `hydrology` | `watersheds_basins` | `s$snap_pour_points(...)` | `wbw_run_tool("snap_pour_points", args = list(...), session = s)` | `Vector` | Vector output |
| `sobel_filter` | `remote_sensing` | `edge_feature_detection` | `s$sobel_filter(...)` | `wbw_run_tool("sobel_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `soil_landscape_classification` | `precision_agriculture` | `general` | `s$soil_landscape_classification(...)` | `wbw_run_tool("soil_landscape_classification", args = list(...), session = s)` | `tuple[Raster, Raster, Vector, str]` | Multiple outputs (tuple) |
| `solar_site_suitability_analysis` | `terrain` | `workflow_products` | `s$solar_site_suitability_analysis(...)` | `wbw_run_tool("solar_site_suitability_analysis", args = list(...), session = s)` | `Any` | See tool docs |
| `sort_lidar` | `lidar` | `io_management` | `s$sort_lidar(...)` | `wbw_run_tool("sort_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `spatial_join` | `vector` | `overlay_analysis` | `s$spatial_join(...)` | `wbw_run_tool("spatial_join", args = list(...), session = s)` | `Vector` | Vector output |
| `spectral_angle_mapper` | `remote_sensing` | `spectral_analytics` | `s$spectral_angle_mapper(...)` | `wbw_run_tool("spectral_angle_mapper", args = list(...), session = s)` | `Any` | See tool docs |
| `spectral_library_matching` | `remote_sensing` | `spectral_analytics` | `s$spectral_library_matching(...)` | `wbw_run_tool("spectral_library_matching", args = list(...), session = s)` | `Any` | See tool docs |
| `spherical_std_dev_of_normals` | `terrain` | `roughness_texture` | `s$spherical_std_dev_of_normals(...)` | `wbw_run_tool("spherical_std_dev_of_normals", args = list(...), session = s)` | `Raster` | Raster output |
| `split_colour_composite` | `remote_sensing` | `enhancement_contrast` | `s$split_colour_composite(...)` | `wbw_run_tool("split_colour_composite", args = list(...), session = s)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `split_lidar` | `lidar` | `io_management` | `s$split_lidar(...)` | `wbw_run_tool("split_lidar", args = list(...), session = s)` | `Lidar` | LiDAR output |
| `split_lines_at_intersections` | `vector` | `network_analysis` | `s$split_lines_at_intersections(...)` | `wbw_run_tool("split_lines_at_intersections", args = list(...), session = s)` | `Any` | See tool docs |
| `split_vector_lines` | `vector` | `geometry_processing` | `s$split_vector_lines(...)` | `wbw_run_tool("split_vector_lines", args = list(...), session = s)` | `Vector` | Vector output |
| `split_with_lines` | `vector` | `geometry_processing` | `s$split_with_lines(...)` | `wbw_run_tool("split_with_lines", args = list(...), session = s)` | `Vector` | Vector output |
| `sqrt` | `raster` | `general` | `s$sqrt(...)` | `wbw_run_tool("sqrt", args = list(...), session = s)` | `Raster` | Raster output |
| `square` | `raster` | `general` | `s$square(...)` | `wbw_run_tool("square", args = list(...), session = s)` | `Raster` | Raster output |
| `standard_deviation_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `s$standard_deviation_contrast_stretch(...)` | `wbw_run_tool("standard_deviation_contrast_stretch", args = list(...), session = s)` | `Raster` | Raster output |
| `standard_deviation_filter` | `remote_sensing` | `filters` | `s$standard_deviation_filter(...)` | `wbw_run_tool("standard_deviation_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `standard_deviation_of_slope` | `terrain` | `roughness_texture` | `s$standard_deviation_of_slope(...)` | `wbw_run_tool("standard_deviation_of_slope", args = list(...), session = s)` | `Raster` | Raster output |
| `standard_deviation_overlay` | `raster` | `overlay_math` | `s$standard_deviation_overlay(...)` | `wbw_run_tool("standard_deviation_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `stochastic_depression_analysis` | `hydrology` | `depressions_storage` | `s$stochastic_depression_analysis(...)` | `wbw_run_tool("stochastic_depression_analysis", args = list(...), session = s)` | `Raster` | Raster output |
| `strahler_order_basins` | `streams` | `ordering_metrics` | `s$strahler_order_basins(...)` | `wbw_run_tool("strahler_order_basins", args = list(...), session = s)` | `Raster` | Raster output |
| `strahler_stream_order` | `streams` | `ordering_metrics` | `s$strahler_stream_order(...)` | `wbw_run_tool("strahler_stream_order", args = list(...), session = s)` | `Raster` | Raster output |
| `stream_link_class` | `streams` | `ordering_metrics` | `s$stream_link_class(...)` | `wbw_run_tool("stream_link_class", args = list(...), session = s)` | `Raster` | Raster output |
| `stream_link_identifier` | `streams` | `ordering_metrics` | `s$stream_link_identifier(...)` | `wbw_run_tool("stream_link_identifier", args = list(...), session = s)` | `Raster` | Raster output |
| `stream_link_length` | `streams` | `ordering_metrics` | `s$stream_link_length(...)` | `wbw_run_tool("stream_link_length", args = list(...), session = s)` | `Raster` | Raster output |
| `stream_link_slope` | `streams` | `ordering_metrics` | `s$stream_link_slope(...)` | `wbw_run_tool("stream_link_slope", args = list(...), session = s)` | `Raster` | Raster output |
| `stream_slope_continuous` | `streams` | `ordering_metrics` | `s$stream_slope_continuous(...)` | `wbw_run_tool("stream_slope_continuous", args = list(...), session = s)` | `Raster` | Raster output |
| `subbasins` | `hydrology` | `watersheds_basins` | `s$subbasins(...)` | `wbw_run_tool("subbasins", args = list(...), session = s)` | `Raster` | Raster output |
| `subtract` | `raster` | `overlay_math` | `s$subtract(...)` | `wbw_run_tool("subtract", args = list(...), session = s)` | `Raster` | Raster output |
| `sum_overlay` | `raster` | `overlay_math` | `s$sum_overlay(...)` | `wbw_run_tool("sum_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `surface_area_ratio` | `terrain` | `general` | `s$surface_area_ratio(...)` | `wbw_run_tool("surface_area_ratio", args = list(...), session = s)` | `Raster` | Raster output |
| `svm_classification` | `remote_sensing` | `classification` | `s$svm_classification(...)` | `wbw_run_tool("svm_classification", args = list(...), session = s)` | `Raster` | Raster output |
| `svm_regression` | `remote_sensing` | `classification` | `s$svm_regression(...)` | `wbw_run_tool("svm_regression", args = list(...), session = s)` | `Raster` | Raster output |
| `symmetrical_difference` | `vector` | `overlay_analysis` | `s$symmetrical_difference(...)` | `wbw_run_tool("symmetrical_difference", args = list(...), session = s)` | `Vector` | Vector output |
| `tan` | `raster` | `general` | `s$tan(...)` | `wbw_run_tool("tan", args = list(...), session = s)` | `Raster` | Raster output |
| `tangential_curvature` | `terrain` | `derivatives` | `s$tangential_curvature(...)` | `wbw_run_tool("tangential_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `tanh` | `raster` | `general` | `s$tanh(...)` | `wbw_run_tool("tanh", args = list(...), session = s)` | `Raster` | Raster output |
| `terrain_constraint_and_conflict_analysis` | `terrain` | `workflow_products` | `s$terrain_constraint_and_conflict_analysis(...)` | `wbw_run_tool("terrain_constraint_and_conflict_analysis", args = list(...), session = s)` | `Any` | See tool docs |
| `terrain_constructability_and_cost_analysis` | `terrain` | `workflow_products` | `s$terrain_constructability_and_cost_analysis(...)` | `wbw_run_tool("terrain_constructability_and_cost_analysis", args = list(...), session = s)` | `Any` | See tool docs |
| `terrain_corrected_optical_analytics` | `remote_sensing` | `radiometric_correction` | `s$terrain_corrected_optical_analytics(...)` | `wbw_run_tool("terrain_corrected_optical_analytics", args = list(...), session = s)` | `Any` | See tool docs |
| `thicken_raster_line` | `remote_sensing` | `filters` | `s$thicken_raster_line(...)` | `wbw_run_tool("thicken_raster_line", args = list(...), session = s)` | `Raster` | Raster output |
| `time_in_daylight` | `terrain` | `visibility` | `s$time_in_daylight(...)` | `wbw_run_tool("time_in_daylight", args = list(...), session = s)` | `Raster` | Raster output |
| `time_series_change_intelligence` | `remote_sensing` | `change_detection` | `s$time_series_change_intelligence(...)` | `wbw_run_tool("time_series_change_intelligence", args = list(...), session = s)` | `tuple[Raster, Raster, Raster, Raster, str]` | Multiple outputs (tuple) |
| `tin_interpolation` | `raster` | `general` | `s$tin_interpolation(...)` | `wbw_run_tool("tin_interpolation", args = list(...), session = s)` | `Raster` | Raster output |
| `to_degrees` | `raster` | `general` | `s$to_degrees(...)` | `wbw_run_tool("to_degrees", args = list(...), session = s)` | `Raster` | Raster output |
| `to_radians` | `raster` | `general` | `s$to_radians(...)` | `wbw_run_tool("to_radians", args = list(...), session = s)` | `Raster` | Raster output |
| `tophat_transform` | `remote_sensing` | `filters` | `s$tophat_transform(...)` | `wbw_run_tool("tophat_transform", args = list(...), session = s)` | `Raster` | Raster output |
| `topo_render` | `terrain` | `workflow_products` | `s$topo_render(...)` | `wbw_run_tool("topo_render", args = list(...), session = s)` | `Raster` | Raster output |
| `topographic_hachures` | `terrain` | `general` | `s$topographic_hachures(...)` | `wbw_run_tool("topographic_hachures", args = list(...), session = s)` | `Vector` | Vector output |
| `topographic_position_animation` | `terrain` | `multiscale_signatures` | `s$topographic_position_animation(...)` | `wbw_run_tool("topographic_position_animation", args = list(...), session = s)` | `tuple[str, str]` | Multiple outputs (tuple) |
| `topological_breach_burn` | `hydrology` | `depressions_storage` | `s$topological_breach_burn(...)` | `wbw_run_tool("topological_breach_burn", args = list(...), session = s)` | `tuple[Raster, Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `topological_stream_order` | `streams` | `ordering_metrics` | `s$topological_stream_order(...)` | `wbw_run_tool("topological_stream_order", args = list(...), session = s)` | `Raster` | Raster output |
| `topology_rule_autofix` | `conversion` | `geometry_topology` | `s$topology_rule_autofix(...)` | `wbw_run_tool("topology_rule_autofix", args = list(...), session = s)` | `Any` | See tool docs |
| `topology_rule_validate` | `conversion` | `geometry_topology` | `s$topology_rule_validate(...)` | `wbw_run_tool("topology_rule_validate", args = list(...), session = s)` | `Any` | See tool docs |
| `topology_validation_report` | `conversion` | `geometry_topology` | `s$topology_validation_report(...)` | `wbw_run_tool("topology_validation_report", args = list(...), session = s)` | `str` | Report/path string output |
| `total_curvature` | `terrain` | `derivatives` | `s$total_curvature(...)` | `wbw_run_tool("total_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `total_filter` | `remote_sensing` | `filters` | `s$total_filter(...)` | `wbw_run_tool("total_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `trace_downslope_flowpaths` | `hydrology` | `flow_routing` | `s$trace_downslope_flowpaths(...)` | `wbw_run_tool("trace_downslope_flowpaths", args = list(...), session = s)` | `Raster` | Raster output |
| `transfer_attributes` | `vector` | `network_analysis` | `s$transfer_attributes(...)` | `wbw_run_tool("transfer_attributes", args = list(...), session = s)` | `Any` | See tool docs |
| `travelling_salesman_problem` | `vector` | `network_analysis` | `s$travelling_salesman_problem(...)` | `wbw_run_tool("travelling_salesman_problem", args = list(...), session = s)` | `Vector` | Vector output |
| `trend_surface` | `raster` | `general` | `s$trend_surface(...)` | `wbw_run_tool("trend_surface", args = list(...), session = s)` | `Raster` | Raster output |
| `trend_surface_vector_points` | `raster` | `general` | `s$trend_surface_vector_points(...)` | `wbw_run_tool("trend_surface_vector_points", args = list(...), session = s)` | `Raster` | Raster output |
| `tributary_identifier` | `streams` | `ordering_metrics` | `s$tributary_identifier(...)` | `wbw_run_tool("tributary_identifier", args = list(...), session = s)` | `Raster` | Raster output |
| `true_colour_composite` | `remote_sensing` | `enhancement_contrast` | `s$true_colour_composite(...)` | `wbw_run_tool("true_colour_composite", args = list(...), session = s)` | `Any` | See tool docs |
| `truncate` | `raster` | `general` | `s$truncate(...)` | `wbw_run_tool("truncate", args = list(...), session = s)` | `Raster` | Raster output |
| `turning_bands_simulation` | `raster` | `general` | `s$turning_bands_simulation(...)` | `wbw_run_tool("turning_bands_simulation", args = list(...), session = s)` | `Raster` | Raster output |
| `two_sample_ks_test` | `raster` | `general` | `s$two_sample_ks_test(...)` | `wbw_run_tool("two_sample_ks_test", args = list(...), session = s)` | `str` | Report/path string output |
| `union` | `vector` | `overlay_analysis` | `s$union(...)` | `wbw_run_tool("union", args = list(...), session = s)` | `Vector` | Vector output |
| `unnest_basins` | `hydrology` | `watersheds_basins` | `s$unnest_basins(...)` | `wbw_run_tool("unnest_basins", args = list(...), session = s)` | `list[Raster]` | Raster output |
| `unsharp_masking` | `remote_sensing` | `filters` | `s$unsharp_masking(...)` | `wbw_run_tool("unsharp_masking", args = list(...), session = s)` | `Raster` | Raster output |
| `unsphericity` | `terrain` | `derivatives` | `s$unsphericity(...)` | `wbw_run_tool("unsphericity", args = list(...), session = s)` | `Raster` | Raster output |
| `update` | `vector` | `overlay_analysis` | `s$update(...)` | `wbw_run_tool("update", args = list(...), session = s)` | `Vector` | Vector output |
| `update_nodata_cells` | `raster` | `overlay_math` | `s$update_nodata_cells(...)` | `wbw_run_tool("update_nodata_cells", args = list(...), session = s)` | `Raster` | Raster output |
| `upslope_depression_storage` | `hydrology` | `depressions_storage` | `s$upslope_depression_storage(...)` | `wbw_run_tool("upslope_depression_storage", args = list(...), session = s)` | `Raster` | Raster output |
| `urban_expansion_impact_assessment` | `terrain` | `workflow_products` | `s$urban_expansion_impact_assessment(...)` | `wbw_run_tool("urban_expansion_impact_assessment", args = list(...), session = s)` | `Any` | See tool docs |
| `user_defined_weights_filter` | `remote_sensing` | `filters` | `s$user_defined_weights_filter(...)` | `wbw_run_tool("user_defined_weights_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `utility_corridor_encroachment_and_access_planning` | `vector` | `workflow_products` | `s$utility_corridor_encroachment_and_access_planning(...)` | `wbw_run_tool("utility_corridor_encroachment_and_access_planning", args = list(...), session = s)` | `Any` | See tool docs |
| `utility_corridor_encroachment_intelligence` | `terrain` | `workflow_products` | `s$utility_corridor_encroachment_intelligence(...)` | `wbw_run_tool("utility_corridor_encroachment_intelligence", args = list(...), session = s)` | `Any` | See tool docs |
| `vector_hex_binning` | `vector` | `sampling_gridding` | `s$vector_hex_binning(...)` | `wbw_run_tool("vector_hex_binning", args = list(...), session = s)` | `Vector` | Vector output |
| `vector_lines_to_raster` | `conversion` | `raster_vector_conversion` | `s$vector_lines_to_raster(...)` | `wbw_run_tool("vector_lines_to_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `vector_points_to_raster` | `conversion` | `raster_vector_conversion` | `s$vector_points_to_raster(...)` | `wbw_run_tool("vector_points_to_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `vector_polygons_to_raster` | `conversion` | `raster_vector_conversion` | `s$vector_polygons_to_raster(...)` | `wbw_run_tool("vector_polygons_to_raster", args = list(...), session = s)` | `Raster` | Raster output |
| `vector_stream_network_analysis` | `streams` | `ordering_metrics` | `s$vector_stream_network_analysis(...)` | `wbw_run_tool("vector_stream_network_analysis", args = list(...), session = s)` | `Vector` | Vector output |
| `vector_summary_statistics` | `conversion` | `vector_table_io` | `s$vector_summary_statistics(...)` | `wbw_run_tool("vector_summary_statistics", args = list(...), session = s)` | `str` | Report/path string output |
| `vehicle_routing_cvrp` | `vector` | `network_analysis` | `s$vehicle_routing_cvrp(...)` | `wbw_run_tool("vehicle_routing_cvrp", args = list(...), session = s)` | `Any` | See tool docs |
| `vehicle_routing_pickup_delivery` | `vector` | `network_analysis` | `s$vehicle_routing_pickup_delivery(...)` | `wbw_run_tool("vehicle_routing_pickup_delivery", args = list(...), session = s)` | `Any` | See tool docs |
| `vehicle_routing_vrptw` | `vector` | `network_analysis` | `s$vehicle_routing_vrptw(...)` | `wbw_run_tool("vehicle_routing_vrptw", args = list(...), session = s)` | `Any` | See tool docs |
| `vertical_excess_curvature` | `terrain` | `derivatives` | `s$vertical_excess_curvature(...)` | `wbw_run_tool("vertical_excess_curvature", args = list(...), session = s)` | `Raster` | Raster output |
| `viewshed` | `terrain` | `visibility` | `s$viewshed(...)` | `wbw_run_tool("viewshed", args = list(...), session = s)` | `Raster` | Raster output |
| `visibility_index` | `terrain` | `visibility` | `s$visibility_index(...)` | `wbw_run_tool("visibility_index", args = list(...), session = s)` | `Raster` | Raster output |
| `voronoi_diagram` | `vector` | `sampling_gridding` | `s$voronoi_diagram(...)` | `wbw_run_tool("voronoi_diagram", args = list(...), session = s)` | `Vector` | Vector output |
| `watershed` | `hydrology` | `watersheds_basins` | `s$watershed(...)` | `wbw_run_tool("watershed", args = list(...), session = s)` | `Raster` | Raster output |
| `watershed_from_raster_pour_points` | `hydrology` | `watersheds_basins` | `s$watershed_from_raster_pour_points(...)` | `wbw_run_tool("watershed_from_raster_pour_points", args = list(...), session = s)` | `Raster` | Raster output |
| `weighted_overlay` | `raster` | `overlay_math` | `s$weighted_overlay(...)` | `wbw_run_tool("weighted_overlay", args = list(...), session = s)` | `Raster` | Raster output |
| `weighted_sum` | `raster` | `overlay_math` | `s$weighted_sum(...)` | `wbw_run_tool("weighted_sum", args = list(...), session = s)` | `Raster` | Raster output |
| `wetland_hydrogeomorphic_classification` | `terrain` | `workflow_products` | `s$wetland_hydrogeomorphic_classification(...)` | `wbw_run_tool("wetland_hydrogeomorphic_classification", args = list(...), session = s)` | `Any` | See tool docs |
| `wetness_index` | `hydrology` | `hydrologic_indices` | `s$wetness_index(...)` | `wbw_run_tool("wetness_index", args = list(...), session = s)` | `Raster` | Raster output |
| `wiener_filter` | `remote_sensing` | `filters` | `s$wiener_filter(...)` | `wbw_run_tool("wiener_filter", args = list(...), session = s)` | `Raster` | Raster output |
| `wilcoxon_signed_rank_test` | `raster` | `general` | `s$wilcoxon_signed_rank_test(...)` | `wbw_run_tool("wilcoxon_signed_rank_test", args = list(...), session = s)` | `str` | Report/path string output |
| `wildfire_fuel_loading_and_risk_matrix` | `terrain` | `workflow_products` | `s$wildfire_fuel_loading_and_risk_matrix(...)` | `wbw_run_tool("wildfire_fuel_loading_and_risk_matrix", args = list(...), session = s)` | `Any` | See tool docs |
| `wind_turbine_siting` | `terrain` | `workflow_products` | `s$wind_turbine_siting(...)` | `wbw_run_tool("wind_turbine_siting", args = list(...), session = s)` | `Any` | See tool docs |
| `wisart_iterative_clustering` | `remote_sensing` | `sar` | `s$wisart_iterative_clustering(...)` | `wbw_run_tool("wisart_iterative_clustering", args = list(...), session = s)` | `Raster` | Raster output |
| `write_function_memory_insertion` | `remote_sensing` | `change_detection` | `s$write_function_memory_insertion(...)` | `wbw_run_tool("write_function_memory_insertion", args = list(...), session = s)` | `Raster` | Raster output |
| `yamaguchi_4component_decomposition` | `remote_sensing` | `sar` | `s$yamaguchi_4component_decomposition(...)` | `wbw_run_tool("yamaguchi_4component_decomposition", args = list(...), session = s)` | `Any` | See tool docs |
| `yield_data_conditioning_and_qa` | `precision_agriculture` | `general` | `s$yield_data_conditioning_and_qa(...)` | `wbw_run_tool("yield_data_conditioning_and_qa", args = list(...), session = s)` | `Any` | See tool docs |
| `z_scores` | `raster` | `general` | `s$z_scores(...)` | `wbw_run_tool("z_scores", args = list(...), session = s)` | `Raster` | Raster output |
| `zonal_statistics` | `raster` | `general` | `s$zonal_statistics(...)` | `wbw_run_tool("zonal_statistics", args = list(...), session = s)` | `Raster` | Raster output |
