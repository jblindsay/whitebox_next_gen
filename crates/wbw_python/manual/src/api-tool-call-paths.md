# Tool Call Paths (Python)

This chapter maps each tool identifier to the preferred call path on WbEnvironment and summarizes outputs.

Call pattern:

- General-subcategory tools: `wbe.category.tool_id(...)`
- Other tools: `wbe.category.subcategory.tool_id(...)`
- Generic execution: `wbe.run_tool("tool_id", args)`

Total tools: 747

| Tool ID | Category | Subcategory | Preferred Call | Return Type | Output Summary |
|---|---|---|---|---|---|
| `abs` | `raster` | `general` | `wbe.raster.abs(...)` | `Raster` | Raster output |
| `accumulation_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.accumulation_curvature(...)` | `Raster` | Raster output |
| `adaptive_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.adaptive_filter(...)` | `Raster` | Raster output |
| `add` | `raster` | `overlay_math` | `wbe.raster.overlay_math.add(...)` | `Raster` | Raster output |
| `add_field` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.add_field(...)` | `Vector` | Vector output |
| `add_geometry_attributes` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.add_geometry_attributes(...)` | `Vector` | Vector output |
| `add_point_coordinates_to_table` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.add_point_coordinates_to_table(...)` | `Vector` | Vector output |
| `aggregate_raster` | `raster` | `general` | `wbe.raster.aggregate_raster(...)` | `Raster` | Raster output |
| `anisotropic_diffusion_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.anisotropic_diffusion_filter(...)` | `Raster` | Raster output |
| `anova` | `raster` | `general` | `wbe.raster.anova(...)` | `str` | Report/path string output |
| `arccos` | `raster` | `general` | `wbe.raster.arccos(...)` | `Raster` | Raster output |
| `arcosh` | `raster` | `general` | `wbe.raster.arcosh(...)` | `Raster` | Raster output |
| `arcsin` | `raster` | `general` | `wbe.raster.arcsin(...)` | `Raster` | Raster output |
| `arctan` | `raster` | `general` | `wbe.raster.arctan(...)` | `Raster` | Raster output |
| `arsinh` | `raster` | `general` | `wbe.raster.arsinh(...)` | `Raster` | Raster output |
| `artanh` | `raster` | `general` | `wbe.raster.artanh(...)` | `Raster` | Raster output |
| `ascii_to_las` | `lidar` | `io_management` | `wbe.lidar.io_management.ascii_to_las(...)` | `Lidar` | LiDAR output |
| `aspect` | `terrain` | `derivatives` | `wbe.terrain.derivatives.aspect(...)` | `Raster` | Raster output |
| `assess_route` | `terrain` | `general` | `wbe.terrain.assess_route(...)` | `Vector` | Vector output |
| `assign_projection_lidar` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.assign_projection_lidar(...)` | `Any` | See tool docs |
| `assign_projection_raster` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.assign_projection_raster(...)` | `Any` | See tool docs |
| `assign_projection_vector` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.assign_projection_vector(...)` | `Any` | See tool docs |
| `atan2` | `raster` | `general` | `wbe.raster.atan2(...)` | `Raster` | Raster output |
| `attribute_correlation` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.attribute_correlation(...)` | `str` | Report/path string output |
| `attribute_histogram` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.attribute_histogram(...)` | `str` | Report/path string output |
| `attribute_scattergram` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.attribute_scattergram(...)` | `str` | Report/path string output |
| `average_flowpath_slope` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.average_flowpath_slope(...)` | `Raster` | Raster output |
| `average_horizon_distance` | `terrain` | `visibility` | `wbe.terrain.visibility.average_horizon_distance(...)` | `Raster` | Raster output |
| `average_normal_vector_angular_deviation` | `terrain` | `roughness_texture` | `wbe.terrain.roughness_texture.average_normal_vector_angular_deviation(...)` | `Raster` | Raster output |
| `average_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.average_overlay(...)` | `Raster` | Raster output |
| `average_upslope_flowpath_length` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.average_upslope_flowpath_length(...)` | `Raster` | Raster output |
| `balance_contrast_enhancement` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.balance_contrast_enhancement(...)` | `Raster` | Raster output |
| `basins` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.basins(...)` | `Raster` | Raster output |
| `bilateral_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.bilateral_filter(...)` | `Raster` | Raster output |
| `block_maximum` | `raster` | `general` | `wbe.raster.block_maximum(...)` | `Raster` | Raster output |
| `block_minimum` | `raster` | `general` | `wbe.raster.block_minimum(...)` | `Raster` | Raster output |
| `bool_and` | `raster` | `overlay_math` | `wbe.raster.overlay_math.bool_and(...)` | `Raster` | Raster output |
| `bool_not` | `raster` | `overlay_math` | `wbe.raster.overlay_math.bool_not(...)` | `Raster` | Raster output |
| `bool_or` | `raster` | `overlay_math` | `wbe.raster.overlay_math.bool_or(...)` | `Raster` | Raster output |
| `bool_xor` | `raster` | `overlay_math` | `wbe.raster.overlay_math.bool_xor(...)` | `Raster` | Raster output |
| `boundary_shape_complexity` | `raster` | `general` | `wbe.raster.boundary_shape_complexity(...)` | `Raster` | Raster output |
| `brdf_normalization` | `remote_sensing` | `radiometric_correction` | `wbe.remote_sensing.radiometric_correction.brdf_normalization(...)` | `Any` | See tool docs |
| `brdf_surface_reflectance_consistency` | `remote_sensing` | `radiometric_correction` | `wbe.remote_sensing.radiometric_correction.brdf_surface_reflectance_consistency(...)` | `tuple[Raster, Raster, Raster, str]` | Multiple outputs (tuple) |
| `breach_depressions_least_cost` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.breach_depressions_least_cost(...)` | `Raster` | Raster output |
| `breach_single_cell_pits` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.breach_single_cell_pits(...)` | `Raster` | Raster output |
| `breakline_mapping` | `terrain` | `general` | `wbe.terrain.breakline_mapping(...)` | `Vector` | Vector output |
| `buffer_raster` | `raster` | `distance_cost` | `wbe.raster.distance_cost.buffer_raster(...)` | `Raster` | Raster output |
| `build_network_topology` | `vector` | `network_analysis` | `wbe.vector.network_analysis.build_network_topology(...)` | `Any` | See tool docs |
| `build_object_hierarchy_multiscale` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.build_object_hierarchy_multiscale(...)` | `str` | Report/path string output |
| `burn_streams` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.burn_streams(...)` | `Raster` | Raster output |
| `burn_streams_at_roads` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.burn_streams_at_roads(...)` | `Raster` | Raster output |
| `canny_edge_detection` | `remote_sensing` | `edge_feature_detection` | `wbe.remote_sensing.edge_feature_detection.canny_edge_detection(...)` | `Raster` | Raster output |
| `carbon_sequestration_verification_audit` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.carbon_sequestration_verification_audit(...)` | `Any` | See tool docs |
| `casorati_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.casorati_curvature(...)` | `Raster` | Raster output |
| `ceil` | `raster` | `general` | `wbe.raster.ceil(...)` | `Raster` | Raster output |
| `centroid_raster` | `raster` | `general` | `wbe.raster.centroid_raster(...)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `centroid_vector` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.centroid_vector(...)` | `Vector` | Vector output |
| `change_vector_analysis` | `remote_sensing` | `change_detection` | `wbe.remote_sensing.change_detection.change_vector_analysis(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `circular_variance_of_aspect` | `terrain` | `roughness_texture` | `wbe.terrain.roughness_texture.circular_variance_of_aspect(...)` | `Raster` | Raster output |
| `classify_buildings_in_lidar` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.classify_buildings_in_lidar(...)` | `Lidar` | LiDAR output |
| `classify_lidar` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.classify_lidar(...)` | `Lidar` | LiDAR output |
| `classify_objects_ensemble_pro` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.classify_objects_ensemble_pro(...)` | `str` | Report/path string output |
| `classify_objects_random_forest` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.classify_objects_random_forest(...)` | `str` | Report/path string output |
| `classify_objects_rules_basic` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.classify_objects_rules_basic(...)` | `str` | Report/path string output |
| `classify_objects_rules_hierarchical` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.classify_objects_rules_hierarchical(...)` | `str` | Report/path string output |
| `classify_objects_svm` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.classify_objects_svm(...)` | `str` | Report/path string output |
| `classify_overlap_points` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.classify_overlap_points(...)` | `Lidar` | LiDAR output |
| `clean_vector` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.clean_vector(...)` | `Vector` | Vector output |
| `clip` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.clip(...)` | `Vector` | Vector output |
| `clip_lidar_to_polygon` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.clip_lidar_to_polygon(...)` | `Lidar` | LiDAR output |
| `clip_raster_to_polygon` | `raster` | `general` | `wbe.raster.clip_raster_to_polygon(...)` | `Raster` | Raster output |
| `closest_facility_network` | `vector` | `network_analysis` | `wbe.vector.network_analysis.closest_facility_network(...)` | `Vector` | Vector output |
| `closing` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.closing(...)` | `Raster` | Raster output |
| `cloude_pottier_decomposition` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.cloude_pottier_decomposition(...)` | `Any` | See tool docs |
| `clump` | `raster` | `general` | `wbe.raster.clump(...)` | `Raster` | Raster output |
| `colourize_based_on_class` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.colourize_based_on_class(...)` | `Lidar` | LiDAR output |
| `colourize_based_on_point_returns` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.colourize_based_on_point_returns(...)` | `Lidar` | LiDAR output |
| `compactness_ratio` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.compactness_ratio(...)` | `Vector` | Vector output |
| `concave_hull` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.concave_hull(...)` | `Vector` | Vector output |
| `conditional_evaluation` | `raster` | `reclass_mask` | `wbe.raster.reclass_mask.conditional_evaluation(...)` | `Raster` | Raster output |
| `conservative_smoothing_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.conservative_smoothing_filter(...)` | `Raster` | Raster output |
| `construct_vector_tin` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.construct_vector_tin(...)` | `Vector` | Vector output |
| `continuum_removal` | `remote_sensing` | `spectral_analytics` | `wbe.remote_sensing.spectral_analytics.continuum_removal(...)` | `Any` | See tool docs |
| `contours_from_points` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.contours_from_points(...)` | `Vector` | Vector output |
| `contours_from_raster` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.contours_from_raster(...)` | `Vector` | Vector output |
| `convergence_index` | `terrain` | `general` | `wbe.terrain.convergence_index(...)` | `Raster` | Raster output |
| `convert_nodata_to_zero` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.convert_nodata_to_zero(...)` | `Raster` | Raster output |
| `corner_detection` | `remote_sensing` | `edge_feature_detection` | `wbe.remote_sensing.edge_feature_detection.corner_detection(...)` | `Raster` | Raster output |
| `correct_vignetting` | `remote_sensing` | `radiometric_correction` | `wbe.remote_sensing.radiometric_correction.correct_vignetting(...)` | `Raster` | Raster output |
| `corridor_mapping_intelligence` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.corridor_mapping_intelligence(...)` | `Any` | See tool docs |
| `cos` | `raster` | `general` | `wbe.raster.cos(...)` | `Raster` | Raster output |
| `cosh` | `raster` | `general` | `wbe.raster.cosh(...)` | `Raster` | Raster output |
| `cost_allocation` | `raster` | `distance_cost` | `wbe.raster.distance_cost.cost_allocation(...)` | `Raster` | Raster output |
| `cost_distance` | `raster` | `distance_cost` | `wbe.raster.distance_cost.cost_distance(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `cost_pathway` | `raster` | `distance_cost` | `wbe.raster.distance_cost.cost_pathway(...)` | `Raster` | Raster output |
| `count_if` | `raster` | `overlay_math` | `wbe.raster.overlay_math.count_if(...)` | `Raster` | Raster output |
| `create_colour_composite` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.create_colour_composite(...)` | `Raster` | Raster output |
| `create_plane` | `raster` | `general` | `wbe.raster.create_plane(...)` | `Raster` | Raster output |
| `crispness_index` | `raster` | `general` | `wbe.raster.crispness_index(...)` | `str` | Report/path string output |
| `cross_tabulation` | `raster` | `general` | `wbe.raster.cross_tabulation(...)` | `str` | Report/path string output |
| `csv_points_to_vector` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.csv_points_to_vector(...)` | `Vector` | Vector output |
| `cumulative_distribution` | `raster` | `general` | `wbe.raster.cumulative_distribution(...)` | `Raster` | Raster output |
| `curvedness` | `terrain` | `derivatives` | `wbe.terrain.derivatives.curvedness(...)` | `Raster` | Raster output |
| `d8_flow_accum` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.d8_flow_accum(...)` | `Raster` | Raster output |
| `d8_mass_flux` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.d8_mass_flux(...)` | `Raster` | Raster output |
| `d8_pointer` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.d8_pointer(...)` | `Raster` | Raster output |
| `dark_object_subtraction` | `remote_sensing` | `radiometric_correction` | `wbe.remote_sensing.radiometric_correction.dark_object_subtraction(...)` | `Any` | See tool docs |
| `dbscan` | `raster` | `general` | `wbe.raster.dbscan(...)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `decrement` | `raster` | `general` | `wbe.raster.decrement(...)` | `Raster` | Raster output |
| `delete_field` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.delete_field(...)` | `Vector` | Vector output |
| `dem_void_filling` | `terrain` | `general` | `wbe.terrain.dem_void_filling(...)` | `Raster` | Raster output |
| `densify_features` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.densify_features(...)` | `Vector` | Vector output |
| `depth_in_sink` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.depth_in_sink(...)` | `Raster` | Raster output |
| `depth_to_water` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.depth_to_water(...)` | `Raster` | Raster output |
| `deviation_from_mean_elevation` | `terrain` | `general` | `wbe.terrain.deviation_from_mean_elevation(...)` | `Raster` | Raster output |
| `deviation_from_regional_direction` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.deviation_from_regional_direction(...)` | `Vector` | Vector output |
| `diff_of_gaussians_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.diff_of_gaussians_filter(...)` | `Raster` | Raster output |
| `difference` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.difference(...)` | `Vector` | Vector output |
| `difference_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.difference_curvature(...)` | `Raster` | Raster output |
| `difference_from_mean_elevation` | `terrain` | `general` | `wbe.terrain.difference_from_mean_elevation(...)` | `Raster` | Raster output |
| `dinf_flow_accum` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.dinf_flow_accum(...)` | `Raster` | Raster output |
| `dinf_mass_flux` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.dinf_mass_flux(...)` | `Raster` | Raster output |
| `dinf_pointer` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.dinf_pointer(...)` | `Raster` | Raster output |
| `direct_decorrelation_stretch` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.direct_decorrelation_stretch(...)` | `Raster` | Raster output |
| `directional_relief` | `terrain` | `general` | `wbe.terrain.directional_relief(...)` | `Raster` | Raster output |
| `dissolve` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.dissolve(...)` | `Vector` | Vector output |
| `distance_to_outlet` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.distance_to_outlet(...)` | `Raster` | Raster output |
| `diversity_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.diversity_filter(...)` | `Raster` | Raster output |
| `divide` | `raster` | `overlay_math` | `wbe.raster.overlay_math.divide(...)` | `Raster` | Raster output |
| `dn_to_toa_reflectance` | `remote_sensing` | `radiometric_correction` | `wbe.remote_sensing.radiometric_correction.dn_to_toa_reflectance(...)` | `Any` | See tool docs |
| `download_osm_vector` | `vector` | `online_data` | `wbe.vector.online_data.download_osm_vector(...)` | `Any` | See tool docs |
| `downslope_distance_to_stream` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.downslope_distance_to_stream(...)` | `Raster` | Raster output |
| `downslope_flowpath_length` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.downslope_flowpath_length(...)` | `Raster` | Raster output |
| `downslope_index` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.downslope_index(...)` | `Raster` | Raster output |
| `edge_contamination` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.edge_contamination(...)` | `Raster` | Raster output |
| `edge_density` | `terrain` | `roughness_texture` | `wbe.terrain.roughness_texture.edge_density(...)` | `Raster` | Raster output |
| `edge_preserving_mean_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.edge_preserving_mean_filter(...)` | `Raster` | Raster output |
| `edge_proportion` | `raster` | `general` | `wbe.raster.edge_proportion(...)` | `Raster` | Raster output |
| `elev_above_pit` | `terrain` | `general` | `wbe.terrain.elev_above_pit(...)` | `Raster` | Raster output |
| `elev_above_pit_dist` | `terrain` | `general` | `wbe.terrain.elev_above_pit_dist(...)` | `Raster` | Raster output |
| `elev_relative_to_min_max` | `terrain` | `landform_indices` | `wbe.terrain.landform_indices.elev_relative_to_min_max(...)` | `Raster` | Raster output |
| `elev_relative_to_watershed_min_max` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.elev_relative_to_watershed_min_max(...)` | `Raster` | Raster output |
| `elevation_above_stream` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.elevation_above_stream(...)` | `Raster` | Raster output |
| `elevation_above_stream_euclidean` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.elevation_above_stream_euclidean(...)` | `Raster` | Raster output |
| `elevation_percentile` | `terrain` | `general` | `wbe.terrain.elevation_percentile(...)` | `Raster` | Raster output |
| `eliminate_coincident_points` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.eliminate_coincident_points(...)` | `Vector` | Vector output |
| `elongation_ratio` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.elongation_ratio(...)` | `Vector` | Vector output |
| `embankment_mapping` | `terrain` | `general` | `wbe.terrain.embankment_mapping(...)` | `tuple[Raster, Raster | None]` | Multiple outputs (tuple) |
| `emboss_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.emboss_filter(...)` | `Raster` | Raster output |
| `emergency_scenario_routing_and_accessibility_simulator` | `vector` | `network_analysis` | `wbe.vector.network_analysis.emergency_scenario_routing_and_accessibility_simulator(...)` | `Any` | See tool docs |
| `enhanced_lee_filter` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.enhanced_lee_filter(...)` | `Raster` | Raster output |
| `equal_to` | `raster` | `general` | `wbe.raster.equal_to(...)` | `Raster` | Raster output |
| `erase` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.erase(...)` | `Vector` | Vector output |
| `erase_polygon_from_lidar` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.erase_polygon_from_lidar(...)` | `Lidar` | LiDAR output |
| `erase_polygon_from_raster` | `raster` | `general` | `wbe.raster.erase_polygon_from_raster(...)` | `Raster` | Raster output |
| `euclidean_allocation` | `raster` | `distance_cost` | `wbe.raster.distance_cost.euclidean_allocation(...)` | `Raster` | Raster output |
| `euclidean_distance` | `raster` | `distance_cost` | `wbe.raster.distance_cost.euclidean_distance(...)` | `Raster` | Raster output |
| `evaluate_object_classification_accuracy` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.evaluate_object_classification_accuracy(...)` | `str` | Report/path string output |
| `evaluate_segmentation_quality_pro` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.evaluate_segmentation_quality_pro(...)` | `str` | Report/path string output |
| `evaluate_training_sites` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.evaluate_training_sites(...)` | `str` | Report/path string output |
| `exp` | `raster` | `general` | `wbe.raster.exp(...)` | `Raster` | Raster output |
| `exp2` | `raster` | `general` | `wbe.raster.exp2(...)` | `Raster` | Raster output |
| `export_table_to_csv` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.export_table_to_csv(...)` | `str` | Report/path string output |
| `exposure_towards_wind_flux` | `terrain` | `general` | `wbe.terrain.exposure_towards_wind_flux(...)` | `Raster` | Raster output |
| `extend_vector_lines` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.extend_vector_lines(...)` | `Vector` | Vector output |
| `extract_by_attribute` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.extract_by_attribute(...)` | `Vector` | Vector output |
| `extract_nodes` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.extract_nodes(...)` | `Vector` | Vector output |
| `extract_raster_values_at_points` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.extract_raster_values_at_points(...)` | `tuple[Vector, str]` | Multiple outputs (tuple) |
| `extract_streams` | `streams` | `network_extraction` | `wbe.streams.network_extraction.extract_streams(...)` | `Raster` | Raster output |
| `extract_valleys` | `streams` | `network_extraction` | `wbe.streams.network_extraction.extract_valleys(...)` | `Raster` | Raster output |
| `false_colour_composite` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.false_colour_composite(...)` | `Any` | See tool docs |
| `farthest_channel_head` | `streams` | `longitudinal_analysis` | `wbe.streams.longitudinal_analysis.farthest_channel_head(...)` | `Raster` | Raster output |
| `fast_almost_gaussian_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.fast_almost_gaussian_filter(...)` | `Raster` | Raster output |
| `fd8_flow_accum` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.fd8_flow_accum(...)` | `Raster` | Raster output |
| `fd8_pointer` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.fd8_pointer(...)` | `Raster` | Raster output |
| `feature_preserving_smoothing` | `terrain` | `general` | `wbe.terrain.feature_preserving_smoothing(...)` | `Raster` | Raster output |
| `feature_preserving_smoothing_multiscale` | `terrain` | `general` | `wbe.terrain.feature_preserving_smoothing_multiscale(...)` | `Any` | See tool docs |
| `fetch_analysis` | `terrain` | `general` | `wbe.terrain.fetch_analysis(...)` | `Raster` | Raster output |
| `fft_random_field` | `raster` | `general` | `wbe.raster.fft_random_field(...)` | `Raster` | Raster output |
| `field_calculator` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.field_calculator(...)` | `Vector` | Vector output |
| `field_trafficability_and_operation_planning` | `precision_agriculture` | `general` | `wbe.precision_agriculture.field_trafficability_and_operation_planning(...)` | `Any` | See tool docs |
| `fill_burn` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.fill_burn(...)` | `Raster` | Raster output |
| `fill_depressions` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.fill_depressions(...)` | `Raster` | Raster output |
| `fill_depressions_planchon_and_darboux` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.fill_depressions_planchon_and_darboux(...)` | `Raster` | Raster output |
| `fill_depressions_wang_and_liu` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.fill_depressions_wang_and_liu(...)` | `Raster` | Raster output |
| `fill_missing_data` | `terrain` | `general` | `wbe.terrain.fill_missing_data(...)` | `Raster` | Raster output |
| `fill_pits` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.fill_pits(...)` | `Raster` | Raster output |
| `filter_lidar` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.filter_lidar(...)` | `Lidar` | LiDAR output |
| `filter_lidar_by_percentile` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.filter_lidar_by_percentile(...)` | `Lidar` | LiDAR output |
| `filter_lidar_by_reference_surface` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.filter_lidar_by_reference_surface(...)` | `Lidar` | LiDAR output |
| `filter_lidar_classes` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.filter_lidar_classes(...)` | `Lidar` | LiDAR output |
| `filter_lidar_noise` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.filter_lidar_noise(...)` | `Lidar` | LiDAR output |
| `filter_lidar_scan_angles` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.filter_lidar_scan_angles(...)` | `Lidar` | LiDAR output |
| `filter_raster_features_by_area` | `raster` | `general` | `wbe.raster.filter_raster_features_by_area(...)` | `Raster` | Raster output |
| `filter_vector_features_by_area` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.filter_vector_features_by_area(...)` | `Vector` | Vector output |
| `find_flightline_edge_points` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.find_flightline_edge_points(...)` | `Lidar` | LiDAR output |
| `find_lowest_or_highest_points` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.find_lowest_or_highest_points(...)` | `Vector` | Vector output |
| `find_main_stem` | `streams` | `longitudinal_analysis` | `wbe.streams.longitudinal_analysis.find_main_stem(...)` | `Raster` | Raster output |
| `find_noflow_cells` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.find_noflow_cells(...)` | `Raster` | Raster output |
| `find_parallel_flow` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.find_parallel_flow(...)` | `Raster` | Raster output |
| `find_patch_edge_cells` | `raster` | `general` | `wbe.raster.find_patch_edge_cells(...)` | `Raster` | Raster output |
| `find_ridges` | `terrain` | `general` | `wbe.terrain.find_ridges(...)` | `Raster` | Raster output |
| `fix_dangling_arcs` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.fix_dangling_arcs(...)` | `Vector` | Vector output |
| `flatten_lakes` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.flatten_lakes(...)` | `Raster` | Raster output |
| `fleet_routing_and_dispatch_optimizer` | `vector` | `network_analysis` | `wbe.vector.network_analysis.fleet_routing_and_dispatch_optimizer(...)` | `Any` | See tool docs |
| `flightline_overlap` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.flightline_overlap(...)` | `Raster` | Raster output |
| `flip_image` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.flip_image(...)` | `Raster` | Raster output |
| `flood_order` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.flood_order(...)` | `Raster` | Raster output |
| `floor` | `raster` | `general` | `wbe.raster.floor(...)` | `Raster` | Raster output |
| `flow_accum_full_workflow` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.flow_accum_full_workflow(...)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `flow_length_diff` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.flow_length_diff(...)` | `Raster` | Raster output |
| `forestry_structure_and_biomass_intelligence` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.forestry_structure_and_biomass_intelligence(...)` | `Any` | See tool docs |
| `frangi_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.frangi_filter(...)` | `Raster` | Raster output |
| `freeman_durden_decomposition` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.freeman_durden_decomposition(...)` | `Any` | See tool docs |
| `frost_filter` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.frost_filter(...)` | `Raster` | Raster output |
| `fuzzy_knn_classification` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.fuzzy_knn_classification(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `gabor_filter_bank` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.gabor_filter_bank(...)` | `Raster` | Raster output |
| `gamma_correction` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.gamma_correction(...)` | `Raster` | Raster output |
| `gamma_map_filter` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.gamma_map_filter(...)` | `Raster` | Raster output |
| `gaussian_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.gaussian_contrast_stretch(...)` | `Raster` | Raster output |
| `gaussian_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.gaussian_curvature(...)` | `Raster` | Raster output |
| `gaussian_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.gaussian_filter(...)` | `Raster` | Raster output |
| `generalize_classified_raster` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.generalize_classified_raster(...)` | `Raster` | Raster output |
| `generalize_with_similarity` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.generalize_with_similarity(...)` | `Raster` | Raster output |
| `generate_network_nodes` | `vector` | `network_analysis` | `wbe.vector.network_analysis.generate_network_nodes(...)` | `Any` | See tool docs |
| `generating_function` | `terrain` | `derivatives` | `wbe.terrain.derivatives.generating_function(...)` | `Raster` | Raster output |
| `geomorphons` | `terrain` | `landform_indices` | `wbe.terrain.landform_indices.geomorphons(...)` | `Raster` | Raster output |
| `georeference_raster_from_control_points` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.georeference_raster_from_control_points(...)` | `Any` | See tool docs |
| `glcm_texture` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.glcm_texture(...)` | `Raster` | Raster output |
| `greater_than` | `raster` | `general` | `wbe.raster.greater_than(...)` | `Raster` | Raster output |
| `guided_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.guided_filter(...)` | `Raster` | Raster output |
| `guided_uav_image_intake_workflow` | `remote_sensing` | `workflow_products` | `wbe.remote_sensing.workflow_products.guided_uav_image_intake_workflow(...)` | `Any` | See tool docs |
| `h_alpha_wisart_classification` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.h_alpha_wisart_classification(...)` | `Raster` | Raster output |
| `hack_stream_order` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.hack_stream_order(...)` | `Raster` | Raster output |
| `heat_map` | `raster` | `general` | `wbe.raster.heat_map(...)` | `Raster` | Raster output |
| `height_above_ground` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.height_above_ground(...)` | `Lidar` | LiDAR output |
| `hexagonal_grid_from_raster_base` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.hexagonal_grid_from_raster_base(...)` | `Vector` | Vector output |
| `hexagonal_grid_from_vector_base` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.hexagonal_grid_from_vector_base(...)` | `Vector` | Vector output |
| `high_pass_bilateral_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.high_pass_bilateral_filter(...)` | `Raster` | Raster output |
| `high_pass_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.high_pass_filter(...)` | `Raster` | Raster output |
| `high_pass_median_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.high_pass_median_filter(...)` | `Raster` | Raster output |
| `highest_position` | `raster` | `overlay_math` | `wbe.raster.overlay_math.highest_position(...)` | `Raster` | Raster output |
| `hillshade` | `terrain` | `general` | `wbe.terrain.hillshade(...)` | `Raster` | Raster output |
| `hillslopes` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.hillslopes(...)` | `Raster` | Raster output |
| `histogram_equalization` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.histogram_equalization(...)` | `Raster` | Raster output |
| `histogram_matching` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.histogram_matching(...)` | `Raster` | Raster output |
| `histogram_matching_two_images` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.histogram_matching_two_images(...)` | `Raster` | Raster output |
| `hole_proportion` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.hole_proportion(...)` | `Vector` | Vector output |
| `horizon_angle` | `terrain` | `visibility` | `wbe.terrain.visibility.horizon_angle(...)` | `Raster` | Raster output |
| `horizon_area` | `terrain` | `visibility` | `wbe.terrain.visibility.horizon_area(...)` | `Raster` | Raster output |
| `horizontal_excess_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.horizontal_excess_curvature(...)` | `Raster` | Raster output |
| `horton_ratios` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.horton_ratios(...)` | `tuple[float, float, float, float, str | None]` | Multiple outputs (tuple) |
| `horton_stream_order` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.horton_stream_order(...)` | `Raster` | Raster output |
| `hydrologic_connectivity` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.hydrologic_connectivity(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `hypsometric_analysis` | `terrain` | `landform_indices` | `wbe.terrain.landform_indices.hypsometric_analysis(...)` | `str` | Report/path string output |
| `hypsometrically_tinted_hillshade` | `terrain` | `general` | `wbe.terrain.hypsometrically_tinted_hillshade(...)` | `Raster` | Raster output |
| `identity` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.identity(...)` | `Vector` | Vector output |
| `idw_interpolation` | `raster` | `general` | `wbe.raster.idw_interpolation(...)` | `Raster` | Raster output |
| `ihs_to_rgb` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.ihs_to_rgb(...)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `image_autocorrelation` | `raster` | `general` | `wbe.raster.image_autocorrelation(...)` | `str` | Report/path string output |
| `image_correlation` | `raster` | `general` | `wbe.raster.image_correlation(...)` | `str` | Report/path string output |
| `image_correlation_neighbourhood_analysis` | `raster` | `local_neighborhood` | `wbe.raster.local_neighborhood.image_correlation_neighbourhood_analysis(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `image_difference_change_detection` | `remote_sensing` | `change_detection` | `wbe.remote_sensing.change_detection.image_difference_change_detection(...)` | `Any` | See tool docs |
| `image_regression` | `raster` | `general` | `wbe.raster.image_regression(...)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `image_segmentation` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.image_segmentation(...)` | `Raster` | Raster output |
| `image_slider` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.image_slider(...)` | `str` | Report/path string output |
| `image_stack_profile` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.image_stack_profile(...)` | `Any` | See tool docs |
| `impoundment_size_index` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.impoundment_size_index(...)` | `tuple[Raster | None, Raster | None, Raster | None, Raster | None, Raster | None]` | Multiple outputs (tuple) |
| `improved_ground_point_filter` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.improved_ground_point_filter(...)` | `Lidar` | LiDAR output |
| `in_season_crop_stress_intervention_planning` | `precision_agriculture` | `general` | `wbe.precision_agriculture.in_season_crop_stress_intervention_planning(...)` | `Any` | See tool docs |
| `increment` | `raster` | `general` | `wbe.raster.increment(...)` | `Raster` | Raster output |
| `individual_tree_detection` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.individual_tree_detection(...)` | `Vector` | Vector output |
| `individual_tree_segmentation` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.individual_tree_segmentation(...)` | `Lidar` | LiDAR output |
| `inplace_add` | `raster` | `general` | `wbe.raster.inplace_add(...)` | `Raster` | Raster output |
| `inplace_divide` | `raster` | `general` | `wbe.raster.inplace_divide(...)` | `Raster` | Raster output |
| `inplace_multiply` | `raster` | `general` | `wbe.raster.inplace_multiply(...)` | `Raster` | Raster output |
| `inplace_subtract` | `raster` | `general` | `wbe.raster.inplace_subtract(...)` | `Raster` | Raster output |
| `insert_dams` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.insert_dams(...)` | `Raster` | Raster output |
| `integer_division` | `raster` | `general` | `wbe.raster.integer_division(...)` | `Raster` | Raster output |
| `integral_image_transform` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.integral_image_transform(...)` | `Raster` | Raster output |
| `intersect` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.intersect(...)` | `Vector` | Vector output |
| `inverse_pca` | `raster` | `general` | `wbe.raster.inverse_pca(...)` | `list[Raster]` | Raster output |
| `is_nodata` | `raster` | `general` | `wbe.raster.is_nodata(...)` | `Raster` | Raster output |
| `isobasins` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.isobasins(...)` | `Raster` | Raster output |
| `jenson_snap_pour_points` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.jenson_snap_pour_points(...)` | `Vector` | Vector output |
| `join_tables` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.join_tables(...)` | `Vector` | Vector output |
| `k_means_clustering` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.k_means_clustering(...)` | `tuple[Raster, int, str | None]` | Multiple outputs (tuple) |
| `k_nearest_mean_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.k_nearest_mean_filter(...)` | `Raster` | Raster output |
| `k_shortest_paths_network` | `vector` | `network_analysis` | `wbe.vector.network_analysis.k_shortest_paths_network(...)` | `Vector` | Vector output |
| `kappa_index` | `raster` | `general` | `wbe.raster.kappa_index(...)` | `str` | Report/path string output |
| `knn_classification` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.knn_classification(...)` | `Raster` | Raster output |
| `knn_regression` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.knn_regression(...)` | `Raster` | Raster output |
| `ks_normality_test` | `raster` | `general` | `wbe.raster.ks_normality_test(...)` | `str` | Report/path string output |
| `kuan_filter` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.kuan_filter(...)` | `Raster` | Raster output |
| `kuwahara_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.kuwahara_filter(...)` | `Raster` | Raster output |
| `land_surface_temperature_single_channel` | `remote_sensing` | `thermal_emissivity` | `wbe.remote_sensing.thermal_emissivity.land_surface_temperature_single_channel(...)` | `Any` | See tool docs |
| `land_surface_temperature_split_window` | `remote_sensing` | `thermal_emissivity` | `wbe.remote_sensing.thermal_emissivity.land_surface_temperature_split_window(...)` | `Any` | See tool docs |
| `landslide_susceptibility_assessment` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.landslide_susceptibility_assessment(...)` | `Any` | See tool docs |
| `laplacian_filter` | `remote_sensing` | `edge_feature_detection` | `wbe.remote_sensing.edge_feature_detection.laplacian_filter(...)` | `Raster` | Raster output |
| `laplacian_of_gaussians_filter` | `remote_sensing` | `edge_feature_detection` | `wbe.remote_sensing.edge_feature_detection.laplacian_of_gaussians_filter(...)` | `Raster` | Raster output |
| `las_to_ascii` | `lidar` | `io_management` | `wbe.lidar.io_management.las_to_ascii(...)` | `str` | Report/path string output |
| `las_to_shapefile` | `lidar` | `io_management` | `wbe.lidar.io_management.las_to_shapefile(...)` | `Vector` | Vector output |
| `layer_footprint_raster` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.layer_footprint_raster(...)` | `Vector` | Vector output |
| `layer_footprint_vector` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.layer_footprint_vector(...)` | `Vector` | Vector output |
| `lee_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.lee_filter(...)` | `Raster` | Raster output |
| `length_of_upstream_channels` | `streams` | `longitudinal_analysis` | `wbe.streams.longitudinal_analysis.length_of_upstream_channels(...)` | `Raster` | Raster output |
| `less_than` | `raster` | `general` | `wbe.raster.less_than(...)` | `Raster` | Raster output |
| `lidar_block_maximum` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_block_maximum(...)` | `Raster` | Raster output |
| `lidar_block_minimum` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_block_minimum(...)` | `Raster` | Raster output |
| `lidar_change_and_disturbance_analysis` | `lidar` | `workflow_products` | `wbe.lidar.workflow_products.lidar_change_and_disturbance_analysis(...)` | `Any` | See tool docs |
| `lidar_classify_subset` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.lidar_classify_subset(...)` | `Lidar` | LiDAR output |
| `lidar_colourize` | `lidar` | `io_management` | `wbe.lidar.io_management.lidar_colourize(...)` | `Lidar` | LiDAR output |
| `lidar_construct_vector_tin` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_construct_vector_tin(...)` | `Vector` | Vector output |
| `lidar_contour` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_contour(...)` | `Vector` | Vector output |
| `lidar_digital_surface_model` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_digital_surface_model(...)` | `Raster` | Raster output |
| `lidar_eigenvalue_features` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_eigenvalue_features(...)` | `str` | Report/path string output |
| `lidar_elevation_slice` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.lidar_elevation_slice(...)` | `Lidar` | LiDAR output |
| `lidar_ground_point_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.lidar_ground_point_filter(...)` | `Lidar` | LiDAR output |
| `lidar_hex_bin` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_hex_bin(...)` | `Vector` | Vector output |
| `lidar_hillshade` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_hillshade(...)` | `Raster` | Raster output |
| `lidar_histogram` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_histogram(...)` | `str` | Report/path string output |
| `lidar_idw_interpolation` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_idw_interpolation(...)` | `Raster` | Raster output |
| `lidar_info` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_info(...)` | `str` | Report/path string output |
| `lidar_join` | `lidar` | `io_management` | `wbe.lidar.io_management.lidar_join(...)` | `Lidar` | LiDAR output |
| `lidar_kappa` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_kappa(...)` | `Raster` | Raster output |
| `lidar_nearest_neighbour_gridding` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_nearest_neighbour_gridding(...)` | `Raster` | Raster output |
| `lidar_point_density` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_point_density(...)` | `Raster` | Raster output |
| `lidar_point_return_analysis` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_point_return_analysis(...)` | `str` | Report/path string output |
| `lidar_point_stats` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_point_stats(...)` | `str` | Report/path string output |
| `lidar_qa_and_confidence` | `lidar` | `workflow_products` | `wbe.lidar.workflow_products.lidar_qa_and_confidence(...)` | `Any` | See tool docs |
| `lidar_radial_basis_function_interpolation` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_radial_basis_function_interpolation(...)` | `Raster` | Raster output |
| `lidar_ransac_planes` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_ransac_planes(...)` | `Lidar` | LiDAR output |
| `lidar_remove_outliers` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.lidar_remove_outliers(...)` | `Lidar` | LiDAR output |
| `lidar_rooftop_analysis` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.lidar_rooftop_analysis(...)` | `Vector` | Vector output |
| `lidar_segmentation` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.lidar_segmentation(...)` | `Lidar` | LiDAR output |
| `lidar_segmentation_based_filter` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.lidar_segmentation_based_filter(...)` | `Lidar` | LiDAR output |
| `lidar_shift` | `lidar` | `io_management` | `wbe.lidar.io_management.lidar_shift(...)` | `Lidar` | LiDAR output |
| `lidar_sibson_interpolation` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_sibson_interpolation(...)` | `Raster` | Raster output |
| `lidar_terrain_product_suite` | `lidar` | `workflow_products` | `wbe.lidar.workflow_products.lidar_terrain_product_suite(...)` | `Any` | See tool docs |
| `lidar_thin` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_thin(...)` | `Lidar` | LiDAR output |
| `lidar_thin_high_density` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_thin_high_density(...)` | `Lidar` | LiDAR output |
| `lidar_tile` | `lidar` | `io_management` | `wbe.lidar.io_management.lidar_tile(...)` | `Lidar` | LiDAR output |
| `lidar_tile_footprint` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_tile_footprint(...)` | `Vector` | Vector output |
| `lidar_tin_gridding` | `lidar` | `interpolation_gridding` | `wbe.lidar.interpolation_gridding.lidar_tin_gridding(...)` | `Raster` | Raster output |
| `lidar_tophat_transform` | `lidar` | `io_management` | `wbe.lidar.io_management.lidar_tophat_transform(...)` | `Lidar` | LiDAR output |
| `line_detection_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.line_detection_filter(...)` | `Raster` | Raster output |
| `line_intersections` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.line_intersections(...)` | `Vector` | Vector output |
| `line_polygon_clip` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.line_polygon_clip(...)` | `Vector` | Vector output |
| `line_thinning` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.line_thinning(...)` | `Raster` | Raster output |
| `linear_spectral_unmixing` | `remote_sensing` | `spectral_analytics` | `wbe.remote_sensing.spectral_analytics.linear_spectral_unmixing(...)` | `Any` | See tool docs |
| `linearity_index` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.linearity_index(...)` | `Vector` | Vector output |
| `lines_to_polygons` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.lines_to_polygons(...)` | `Vector` | Vector output |
| `list_unique_values` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.list_unique_values(...)` | `str` | Report/path string output |
| `list_unique_values_raster` | `raster` | `general` | `wbe.raster.list_unique_values_raster(...)` | `str` | Report/path string output |
| `ln` | `raster` | `general` | `wbe.raster.ln(...)` | `Raster` | Raster output |
| `local_hypsometric_analysis` | `terrain` | `general` | `wbe.terrain.local_hypsometric_analysis(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `locate_points_along_routes` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.locate_points_along_routes(...)` | `Vector` | Vector output |
| `location_allocation_network` | `vector` | `network_analysis` | `wbe.vector.network_analysis.location_allocation_network(...)` | `Vector` | Vector output |
| `log10` | `raster` | `general` | `wbe.raster.log10(...)` | `Raster` | Raster output |
| `log2` | `raster` | `general` | `wbe.raster.log2(...)` | `Raster` | Raster output |
| `logistic_regression` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.logistic_regression(...)` | `Raster` | Raster output |
| `long_profile` | `streams` | `longitudinal_analysis` | `wbe.streams.longitudinal_analysis.long_profile(...)` | `str | None` | Report/path string output |
| `long_profile_from_points` | `streams` | `longitudinal_analysis` | `wbe.streams.longitudinal_analysis.long_profile_from_points(...)` | `str | None` | Report/path string output |
| `longest_flowpath` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.longest_flowpath(...)` | `Vector` | Vector output |
| `low_points_on_headwater_divides` | `terrain` | `general` | `wbe.terrain.low_points_on_headwater_divides(...)` | `Vector` | Vector output |
| `lowest_position` | `raster` | `overlay_math` | `wbe.raster.overlay_math.lowest_position(...)` | `Raster` | Raster output |
| `majority_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.majority_filter(...)` | `Raster` | Raster output |
| `map_features` | `raster` | `general` | `wbe.raster.map_features(...)` | `Raster` | Raster output |
| `map_matching_v1` | `vector` | `network_analysis` | `wbe.vector.network_analysis.map_matching_v1(...)` | `Vector` | Vector output |
| `map_off_terrain_objects` | `terrain` | `general` | `wbe.terrain.map_off_terrain_objects(...)` | `Raster` | Raster output |
| `market_access_and_site_intelligence_workflow` | `vector` | `network_analysis` | `wbe.vector.network_analysis.market_access_and_site_intelligence_workflow(...)` | `Any` | See tool docs |
| `max` | `raster` | `general` | `wbe.raster.max(...)` | `Raster` | Raster output |
| `max_absolute_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.max_absolute_overlay(...)` | `Raster` | Raster output |
| `max_anisotropy_dev` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.max_anisotropy_dev(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `max_anisotropy_dev_signature` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.max_anisotropy_dev_signature(...)` | `str` | Report/path string output |
| `max_branch_length` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.max_branch_length(...)` | `Raster` | Raster output |
| `max_difference_from_mean` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.max_difference_from_mean(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `max_downslope_elev_change` | `terrain` | `general` | `wbe.terrain.max_downslope_elev_change(...)` | `Raster` | Raster output |
| `max_elev_dev_signature` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.max_elev_dev_signature(...)` | `str` | Report/path string output |
| `max_elevation_deviation` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.max_elevation_deviation(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `max_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.max_overlay(...)` | `Raster` | Raster output |
| `max_upslope_elev_change` | `terrain` | `general` | `wbe.terrain.max_upslope_elev_change(...)` | `Raster` | Raster output |
| `max_upslope_flowpath_length` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.max_upslope_flowpath_length(...)` | `Raster` | Raster output |
| `max_upslope_value` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.max_upslope_value(...)` | `Raster` | Raster output |
| `maximal_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.maximal_curvature(...)` | `Raster` | Raster output |
| `maximum_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.maximum_filter(...)` | `Raster` | Raster output |
| `mdinf_flow_accum` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.mdinf_flow_accum(...)` | `Raster` | Raster output |
| `mean_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.mean_curvature(...)` | `Raster` | Raster output |
| `mean_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.mean_filter(...)` | `Raster` | Raster output |
| `median_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.median_filter(...)` | `Raster` | Raster output |
| `medoid` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.medoid(...)` | `Vector` | Vector output |
| `merge_line_segments` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.merge_line_segments(...)` | `Vector` | Vector output |
| `merge_table_with_csv` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.merge_table_with_csv(...)` | `Vector` | Vector output |
| `merge_vectors` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.merge_vectors(...)` | `Vector` | Vector output |
| `min` | `raster` | `general` | `wbe.raster.min(...)` | `Raster` | Raster output |
| `min_absolute_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.min_absolute_overlay(...)` | `Raster` | Raster output |
| `min_dist_classification` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.min_dist_classification(...)` | `Raster` | Raster output |
| `min_downslope_elev_change` | `terrain` | `general` | `wbe.terrain.min_downslope_elev_change(...)` | `Raster` | Raster output |
| `min_max_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.min_max_contrast_stretch(...)` | `Raster` | Raster output |
| `min_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.min_overlay(...)` | `Raster` | Raster output |
| `mine_site_reclamation_compliance_tracker` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.mine_site_reclamation_compliance_tracker(...)` | `Any` | See tool docs |
| `minimal_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.minimal_curvature(...)` | `Raster` | Raster output |
| `minimal_dispersion_flow_algorithm` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.minimal_dispersion_flow_algorithm(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `minimum_bounding_box` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.minimum_bounding_box(...)` | `Vector` | Vector output |
| `minimum_bounding_circle` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.minimum_bounding_circle(...)` | `Vector` | Vector output |
| `minimum_bounding_envelope` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.minimum_bounding_envelope(...)` | `Vector` | Vector output |
| `minimum_convex_hull` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.minimum_convex_hull(...)` | `Vector` | Vector output |
| `minimum_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.minimum_filter(...)` | `Raster` | Raster output |
| `minimum_noise_fraction` | `remote_sensing` | `spectral_analytics` | `wbe.remote_sensing.spectral_analytics.minimum_noise_fraction(...)` | `Any` | See tool docs |
| `modified_k_means_clustering` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.modified_k_means_clustering(...)` | `tuple[Raster, int, str | None]` | Multiple outputs (tuple) |
| `modified_shepard_interpolation` | `raster` | `general` | `wbe.raster.modified_shepard_interpolation(...)` | `Raster` | Raster output |
| `modify_lidar` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.modify_lidar(...)` | `Lidar` | LiDAR output |
| `modify_nodata_value` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.modify_nodata_value(...)` | `Raster` | Raster output |
| `modulo` | `raster` | `overlay_math` | `wbe.raster.overlay_math.modulo(...)` | `Raster` | Raster output |
| `mosaic` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.mosaic(...)` | `Raster` | Raster output |
| `mosaic_with_feathering` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.mosaic_with_feathering(...)` | `Raster` | Raster output |
| `multi_sensor_fusion_monitoring` | `remote_sensing` | `workflow_products` | `wbe.remote_sensing.workflow_products.multi_sensor_fusion_monitoring(...)` | `Any` | See tool docs |
| `multidirectional_hillshade` | `terrain` | `general` | `wbe.terrain.multidirectional_hillshade(...)` | `Raster` | Raster output |
| `multimodal_od_cost_matrix` | `vector` | `network_analysis` | `wbe.vector.network_analysis.multimodal_od_cost_matrix(...)` | `Any` | See tool docs |
| `multimodal_routes_from_od` | `vector` | `network_analysis` | `wbe.vector.network_analysis.multimodal_routes_from_od(...)` | `Any` | See tool docs |
| `multimodal_shortest_path` | `vector` | `network_analysis` | `wbe.vector.network_analysis.multimodal_shortest_path(...)` | `Any` | See tool docs |
| `multipart_to_singlepart` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.multipart_to_singlepart(...)` | `Vector` | Vector output |
| `multiply` | `raster` | `overlay_math` | `wbe.raster.overlay_math.multiply(...)` | `Raster` | Raster output |
| `multiply_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.multiply_overlay(...)` | `Raster` | Raster output |
| `multiscale_curvatures` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_curvatures(...)` | `Raster` | Raster output |
| `multiscale_elevated_index` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_elevated_index(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_elevation_percentile` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_elevation_percentile(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_low_lying_index` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_low_lying_index(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_roughness` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_roughness(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_roughness_signature` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_roughness_signature(...)` | `str` | Report/path string output |
| `multiscale_std_dev_normals` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_std_dev_normals(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `multiscale_std_dev_normals_signature` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_std_dev_normals_signature(...)` | `str` | Report/path string output |
| `multiscale_topographic_position_class` | `terrain` | `landform_indices` | `wbe.terrain.landform_indices.multiscale_topographic_position_class(...)` | `Raster` | Raster output |
| `multiscale_topographic_position_image` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.multiscale_topographic_position_image(...)` | `Raster` | Raster output |
| `narrowness_index` | `raster` | `general` | `wbe.raster.narrowness_index(...)` | `Raster` | Raster output |
| `narrowness_index_vector` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.narrowness_index_vector(...)` | `Vector` | Vector output |
| `natural_neighbour_interpolation` | `raster` | `local_neighborhood` | `wbe.raster.local_neighborhood.natural_neighbour_interpolation(...)` | `Raster` | Raster output |
| `ndvi_based_emissivity` | `remote_sensing` | `thermal_emissivity` | `wbe.remote_sensing.thermal_emissivity.ndvi_based_emissivity(...)` | `Any` | See tool docs |
| `near` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.near(...)` | `Vector` | Vector output |
| `nearest_neighbour_interpolation` | `raster` | `local_neighborhood` | `wbe.raster.local_neighborhood.nearest_neighbour_interpolation(...)` | `Raster` | Raster output |
| `negate` | `raster` | `general` | `wbe.raster.negate(...)` | `Raster` | Raster output |
| `network_accessibility_metrics` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_accessibility_metrics(...)` | `Any` | See tool docs |
| `network_centrality_metrics` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_centrality_metrics(...)` | `Any` | See tool docs |
| `network_connected_components` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_connected_components(...)` | `Vector` | Vector output |
| `network_node_degree` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_node_degree(...)` | `Vector` | Vector output |
| `network_od_cost_matrix` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_od_cost_matrix(...)` | `str` | Report/path string output |
| `network_readiness_and_diagnostics_intelligence` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_readiness_and_diagnostics_intelligence(...)` | `Any` | See tool docs |
| `network_routes_from_od` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_routes_from_od(...)` | `Vector` | Vector output |
| `network_service_area` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_service_area(...)` | `Vector` | Vector output |
| `network_topology_audit` | `vector` | `network_analysis` | `wbe.vector.network_analysis.network_topology_audit(...)` | `Vector` | Vector output |
| `new_raster_from_base_raster` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.new_raster_from_base_raster(...)` | `Raster` | Raster output |
| `new_raster_from_base_vector` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.new_raster_from_base_vector(...)` | `Raster` | Raster output |
| `nibble` | `raster` | `general` | `wbe.raster.nibble(...)` | `Raster` | Raster output |
| `nnd_classification` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.nnd_classification(...)` | `Raster` | Raster output |
| `non_local_means_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.non_local_means_filter(...)` | `Raster` | Raster output |
| `normal_vectors` | `lidar` | `analysis_metrics` | `wbe.lidar.analysis_metrics.normal_vectors(...)` | `Lidar` | LiDAR output |
| `normalize_lidar` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.normalize_lidar(...)` | `Lidar` | LiDAR output |
| `normalized_difference_index` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.normalized_difference_index(...)` | `Raster` | Raster output |
| `not_equal_to` | `raster` | `general` | `wbe.raster.not_equal_to(...)` | `Raster` | Raster output |
| `num_downslope_neighbours` | `terrain` | `general` | `wbe.terrain.num_downslope_neighbours(...)` | `Raster` | Raster output |
| `num_inflowing_neighbours` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.num_inflowing_neighbours(...)` | `Raster` | Raster output |
| `num_upslope_neighbours` | `terrain` | `general` | `wbe.terrain.num_upslope_neighbours(...)` | `Raster` | Raster output |
| `obia_audit_report_pro` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.obia_audit_report_pro(...)` | `str` | Report/path string output |
| `obia_batch_orchestrator_pro` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.obia_batch_orchestrator_pro(...)` | `dict[str, Any]` | Report/path string output |
| `obia_pipeline_basic` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.obia_pipeline_basic(...)` | `dict[str, Any]` | Report/path string output |
| `object_class_probability_maps` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.object_class_probability_maps(...)` | `str` | Report/path string output |
| `object_features_context_neighbors` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.object_features_context_neighbors(...)` | `str` | Report/path string output |
| `object_features_shape_basic` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.object_features_shape_basic(...)` | `str` | Report/path string output |
| `object_features_spectral_basic` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.object_features_spectral_basic(...)` | `str` | Report/path string output |
| `object_features_texture_glcm_basic` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.object_features_texture_glcm_basic(...)` | `str` | Report/path string output |
| `object_features_topology_relations` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.object_features_topology_relations(...)` | `str` | Report/path string output |
| `object_uncertainty_diagnostics_pro` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.object_uncertainty_diagnostics_pro(...)` | `str` | Report/path string output |
| `objects_boundary_refinement_pro` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.objects_boundary_refinement_pro(...)` | `Raster` | Raster output |
| `objects_enforce_min_mapping_unit` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.objects_enforce_min_mapping_unit(...)` | `Raster` | Raster output |
| `od_sensitivity_analysis` | `vector` | `network_analysis` | `wbe.vector.network_analysis.od_sensitivity_analysis(...)` | `Any` | See tool docs |
| `olympic_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.olympic_filter(...)` | `Raster` | Raster output |
| `opening` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.opening(...)` | `Raster` | Raster output |
| `openness` | `terrain` | `visibility` | `wbe.terrain.visibility.openness(...)` | `tuple[Raster, Raster]` | Multiple outputs (tuple) |
| `orthorectification` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.orthorectification(...)` | `Any` | See tool docs |
| `otsu_thresholding` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.otsu_thresholding(...)` | `Raster` | Raster output |
| `paired_sample_t_test` | `raster` | `general` | `wbe.raster.paired_sample_t_test(...)` | `str` | Report/path string output |
| `panchromatic_sharpening` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.panchromatic_sharpening(...)` | `Raster` | Raster output |
| `parallelepiped_classification` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.parallelepiped_classification(...)` | `Raster` | Raster output |
| `parcel_and_land_fabric_topology_compliance_workflow` | `vector` | `workflow_products` | `wbe.vector.workflow_products.parcel_and_land_fabric_topology_compliance_workflow(...)` | `Any` | See tool docs |
| `patch_orientation` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.patch_orientation(...)` | `Vector` | Vector output |
| `pca_based_change_detection` | `remote_sensing` | `change_detection` | `wbe.remote_sensing.change_detection.pca_based_change_detection(...)` | `Any` | See tool docs |
| `pennock_landform_classification` | `terrain` | `landform_indices` | `wbe.terrain.landform_indices.pennock_landform_classification(...)` | `tuple[Raster, str]` | Multiple outputs (tuple) |
| `percent_elev_range` | `terrain` | `landform_indices` | `wbe.terrain.landform_indices.percent_elev_range(...)` | `Raster` | Raster output |
| `percent_equal_to` | `raster` | `overlay_math` | `wbe.raster.overlay_math.percent_equal_to(...)` | `Raster` | Raster output |
| `percent_greater_than` | `raster` | `overlay_math` | `wbe.raster.overlay_math.percent_greater_than(...)` | `Raster` | Raster output |
| `percent_less_than` | `raster` | `overlay_math` | `wbe.raster.overlay_math.percent_less_than(...)` | `Raster` | Raster output |
| `percentage_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.percentage_contrast_stretch(...)` | `Raster` | Raster output |
| `percentile_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.percentile_filter(...)` | `Raster` | Raster output |
| `perimeter_area_ratio` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.perimeter_area_ratio(...)` | `Vector` | Vector output |
| `phi_coefficient` | `raster` | `general` | `wbe.raster.phi_coefficient(...)` | `str` | Report/path string output |
| `pick_from_list` | `raster` | `overlay_math` | `wbe.raster.overlay_math.pick_from_list(...)` | `Raster` | Raster output |
| `piecewise_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.piecewise_contrast_stretch(...)` | `Raster` | Raster output |
| `plan_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.plan_curvature(...)` | `Raster` | Raster output |
| `points_along_lines` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.points_along_lines(...)` | `Vector` | Vector output |
| `polygon_area` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.polygon_area(...)` | `Vector` | Vector output |
| `polygon_long_axis` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.polygon_long_axis(...)` | `Vector` | Vector output |
| `polygon_perimeter` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.polygon_perimeter(...)` | `Vector` | Vector output |
| `polygon_short_axis` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.polygon_short_axis(...)` | `Vector` | Vector output |
| `polygonize` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.polygonize(...)` | `Vector` | Vector output |
| `polygons_to_lines` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.polygons_to_lines(...)` | `Vector` | Vector output |
| `polygons_to_segments` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.polygons_to_segments(...)` | `Raster` | Raster output |
| `post_classification_change` | `remote_sensing` | `change_detection` | `wbe.remote_sensing.change_detection.post_classification_change(...)` | `Any` | See tool docs |
| `power` | `raster` | `overlay_math` | `wbe.raster.overlay_math.power(...)` | `Raster` | Raster output |
| `precision_ag_yield_zone_intelligence` | `precision_agriculture` | `general` | `wbe.precision_agriculture.precision_ag_yield_zone_intelligence(...)` | `Any` | See tool docs |
| `precision_irrigation_optimization` | `precision_agriculture` | `general` | `wbe.precision_agriculture.precision_irrigation_optimization(...)` | `Any` | See tool docs |
| `prewitt_filter` | `remote_sensing` | `edge_feature_detection` | `wbe.remote_sensing.edge_feature_detection.prewitt_filter(...)` | `Raster` | Raster output |
| `principal_component_analysis` | `raster` | `general` | `wbe.raster.principal_component_analysis(...)` | `list[Raster]` | Raster output |
| `principal_curvature_direction` | `terrain` | `derivatives` | `wbe.terrain.derivatives.principal_curvature_direction(...)` | `Raster` | Raster output |
| `print_geotiff_tags` | `raster` | `general` | `wbe.raster.print_geotiff_tags(...)` | `str` | Report/path string output |
| `profile` | `terrain` | `general` | `wbe.terrain.profile(...)` | `str` | Report/path string output |
| `profile_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.profile_curvature(...)` | `Raster` | Raster output |
| `propagate_labels_across_hierarchy` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.propagate_labels_across_hierarchy(...)` | `str` | Report/path string output |
| `prune_vector_streams` | `streams` | `network_extraction` | `wbe.streams.network_extraction.prune_vector_streams(...)` | `Vector` | Vector output |
| `qin_flow_accumulation` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.qin_flow_accumulation(...)` | `Raster` | Raster output |
| `quantiles` | `raster` | `general` | `wbe.raster.quantiles(...)` | `Raster` | Raster output |
| `quinn_flow_accumulation` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.quinn_flow_accumulation(...)` | `Raster` | Raster output |
| `radial_basis_function_interpolation` | `raster` | `general` | `wbe.raster.radial_basis_function_interpolation(...)` | `Raster` | Raster output |
| `radius_of_gyration` | `raster` | `general` | `wbe.raster.radius_of_gyration(...)` | `Raster` | Raster output |
| `raise_walls` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.raise_walls(...)` | `Raster` | Raster output |
| `random_field` | `raster` | `general` | `wbe.raster.random_field(...)` | `Raster` | Raster output |
| `random_forest_classification` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.random_forest_classification(...)` | `Raster` | Raster output |
| `random_forest_classification_fit` | `raster` | `general` | `wbe.raster.random_forest_classification_fit(...)` | `list[int]` | See tool docs |
| `random_forest_classification_predict` | `raster` | `general` | `wbe.raster.random_forest_classification_predict(...)` | `Raster` | Raster output |
| `random_forest_regression` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.random_forest_regression(...)` | `Raster` | Raster output |
| `random_forest_regression_fit` | `raster` | `general` | `wbe.raster.random_forest_regression_fit(...)` | `list[int]` | See tool docs |
| `random_forest_regression_predict` | `raster` | `general` | `wbe.raster.random_forest_regression_predict(...)` | `Raster` | Raster output |
| `random_points_in_polygon` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.random_points_in_polygon(...)` | `Vector` | Vector output |
| `random_sample` | `raster` | `general` | `wbe.raster.random_sample(...)` | `Raster` | Raster output |
| `range_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.range_filter(...)` | `Raster` | Raster output |
| `raster_area` | `raster` | `general` | `wbe.raster.raster_area(...)` | `Raster` | Raster output |
| `raster_calculator` | `raster` | `general` | `wbe.raster.raster_calculator(...)` | `Raster` | Raster output |
| `raster_cell_assignment` | `raster` | `general` | `wbe.raster.raster_cell_assignment(...)` | `Raster` | Raster output |
| `raster_histogram` | `raster` | `general` | `wbe.raster.raster_histogram(...)` | `str` | Report/path string output |
| `raster_perimeter` | `raster` | `general` | `wbe.raster.raster_perimeter(...)` | `Raster` | Raster output |
| `raster_streams_to_vector` | `streams` | `network_extraction` | `wbe.streams.network_extraction.raster_streams_to_vector(...)` | `Vector` | Vector output |
| `raster_summary_stats` | `raster` | `general` | `wbe.raster.raster_summary_stats(...)` | `str` | Report/path string output |
| `raster_to_vector_lines` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.raster_to_vector_lines(...)` | `Vector` | Vector output |
| `raster_to_vector_points` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.raster_to_vector_points(...)` | `Vector` | Vector output |
| `raster_to_vector_polygons` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.raster_to_vector_polygons(...)` | `Vector` | Vector output |
| `rasterize_streams` | `streams` | `network_extraction` | `wbe.streams.network_extraction.rasterize_streams(...)` | `Raster` | Raster output |
| `reciprocal` | `raster` | `general` | `wbe.raster.reciprocal(...)` | `Raster` | Raster output |
| `reclass` | `raster` | `reclass_mask` | `wbe.raster.reclass_mask.reclass(...)` | `Raster` | Raster output |
| `reclass_equal_interval` | `raster` | `reclass_mask` | `wbe.raster.reclass_mask.reclass_equal_interval(...)` | `Raster` | Raster output |
| `recover_flightline_info` | `lidar` | `io_management` | `wbe.lidar.io_management.recover_flightline_info(...)` | `Lidar` | LiDAR output |
| `rectangular_grid_from_raster_base` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.rectangular_grid_from_raster_base(...)` | `Vector` | Vector output |
| `rectangular_grid_from_vector_base` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.rectangular_grid_from_vector_base(...)` | `Vector` | Vector output |
| `refined_lee_filter` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.refined_lee_filter(...)` | `Raster` | Raster output |
| `registration_oriented_feature_workflow` | `remote_sensing` | `workflow_products` | `wbe.remote_sensing.workflow_products.registration_oriented_feature_workflow(...)` | `Any` | See tool docs |
| `reinitialize_attribute_table` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.reinitialize_attribute_table(...)` | `Vector` | Vector output |
| `related_circumscribing_circle` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.related_circumscribing_circle(...)` | `Vector` | Vector output |
| `relative_aspect` | `terrain` | `derivatives` | `wbe.terrain.derivatives.relative_aspect(...)` | `Raster` | Raster output |
| `relative_stream_power_index` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.relative_stream_power_index(...)` | `Raster` | Raster output |
| `relative_topographic_position` | `terrain` | `landform_indices` | `wbe.terrain.landform_indices.relative_topographic_position(...)` | `Raster` | Raster output |
| `remote_sensing_change_detection` | `remote_sensing` | `change_detection` | `wbe.remote_sensing.change_detection.remote_sensing_change_detection(...)` | `tuple[Raster, Raster, str]` | Multiple outputs (tuple) |
| `remove_duplicates` | `lidar` | `filtering_classification` | `wbe.lidar.filtering_classification.remove_duplicates(...)` | `Lidar` | LiDAR output |
| `remove_off_terrain_objects` | `terrain` | `general` | `wbe.terrain.remove_off_terrain_objects(...)` | `Raster` | Raster output |
| `remove_polygon_holes` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.remove_polygon_holes(...)` | `Vector` | Vector output |
| `remove_raster_polygon_holes` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.remove_raster_polygon_holes(...)` | `Raster` | Raster output |
| `remove_short_streams` | `streams` | `network_extraction` | `wbe.streams.network_extraction.remove_short_streams(...)` | `Raster` | Raster output |
| `remove_spurs` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.remove_spurs(...)` | `Raster` | Raster output |
| `rename_field` | `vector` | `attribute_analysis` | `wbe.vector.attribute_analysis.rename_field(...)` | `Vector` | Vector output |
| `repair_stream_vector_topology` | `streams` | `network_extraction` | `wbe.streams.network_extraction.repair_stream_vector_topology(...)` | `Vector` | Vector output |
| `representative_point_vector` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.representative_point_vector(...)` | `Any` | See tool docs |
| `reproject_lidar` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.reproject_lidar(...)` | `Any` | See tool docs |
| `reproject_raster` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.reproject_raster(...)` | `Any` | See tool docs |
| `reproject_vector` | `projection_georeferencing` | `general` | `wbe.projection_georeferencing.reproject_vector(...)` | `Any` | See tool docs |
| `resample` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.resample(...)` | `Raster` | Raster output |
| `rescale_value_range` | `raster` | `general` | `wbe.raster.rescale_value_range(...)` | `Raster` | Raster output |
| `rgb_to_ihs` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.rgb_to_ihs(...)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `rho8_flow_accum` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.rho8_flow_accum(...)` | `Raster` | Raster output |
| `rho8_pointer` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.rho8_pointer(...)` | `Raster` | Raster output |
| `ridge_and_valley_vectors` | `terrain` | `general` | `wbe.terrain.ridge_and_valley_vectors(...)` | `tuple[Vector, Vector]` | Multiple outputs (tuple) |
| `ring_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.ring_curvature(...)` | `Raster` | Raster output |
| `river_centerlines` | `streams` | `network_extraction` | `wbe.streams.network_extraction.river_centerlines(...)` | `Vector` | Vector output |
| `river_corridor_health_assessment` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.river_corridor_health_assessment(...)` | `Any` | See tool docs |
| `roberts_cross_filter` | `remote_sensing` | `edge_feature_detection` | `wbe.remote_sensing.edge_feature_detection.roberts_cross_filter(...)` | `Raster` | Raster output |
| `root_mean_square_error` | `raster` | `general` | `wbe.raster.root_mean_square_error(...)` | `str` | Report/path string output |
| `rotor` | `terrain` | `derivatives` | `wbe.terrain.derivatives.rotor(...)` | `Raster` | Raster output |
| `round` | `raster` | `general` | `wbe.raster.round(...)` | `Raster` | Raster output |
| `route_calibrate` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_calibrate(...)` | `Any` | See tool docs |
| `route_event_governance_for_linear_assets` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_governance_for_linear_assets(...)` | `Any` | See tool docs |
| `route_event_lines_from_layer` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_lines_from_layer(...)` | `Vector` | Vector output |
| `route_event_lines_from_table` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_lines_from_table(...)` | `Vector` | Vector output |
| `route_event_merge` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_merge(...)` | `Any` | See tool docs |
| `route_event_overlay` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_overlay(...)` | `Any` | See tool docs |
| `route_event_points_from_layer` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_points_from_layer(...)` | `Vector` | Vector output |
| `route_event_points_from_table` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_points_from_table(...)` | `Vector` | Vector output |
| `route_event_split` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_event_split(...)` | `Any` | See tool docs |
| `route_measure_qa` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_measure_qa(...)` | `Any` | See tool docs |
| `route_recalibrate` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.route_recalibrate(...)` | `Any` | See tool docs |
| `ruggedness_index` | `terrain` | `roughness_texture` | `wbe.terrain.roughness_texture.ruggedness_index(...)` | `Raster` | Raster output |
| `sar_analysis_readiness` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.sar_analysis_readiness(...)` | `Any` | See tool docs |
| `sar_coregistration` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.sar_coregistration(...)` | `Any` | See tool docs |
| `sar_interferogram_coherence` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.sar_interferogram_coherence(...)` | `Any` | See tool docs |
| `savitzky_golay_2d_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.savitzky_golay_2d_filter(...)` | `Raster` | Raster output |
| `scharr_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.scharr_filter(...)` | `Raster` | Raster output |
| `sediment_transport_index` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.sediment_transport_index(...)` | `Raster` | Raster output |
| `segment_graph_felzenszwalb` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segment_graph_felzenszwalb(...)` | `Raster` | Raster output |
| `segment_multiresolution_hierarchical` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segment_multiresolution_hierarchical(...)` | `tuple[Raster, Raster, str]` | Multiple outputs (tuple) |
| `segment_scale_parameter_optimizer` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segment_scale_parameter_optimizer(...)` | `str` | Report/path string output |
| `segment_slic_superpixels` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segment_slic_superpixels(...)` | `Raster` | Raster output |
| `segment_watershed_markers` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segment_watershed_markers(...)` | `Raster` | Raster output |
| `segments_merge_small_regions` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segments_merge_small_regions(...)` | `Raster` | Raster output |
| `segments_split_low_cohesion` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segments_split_low_cohesion(...)` | `Raster` | Raster output |
| `segments_to_polygons` | `remote_sensing` | `obia` | `wbe.remote_sensing.obia.segments_to_polygons(...)` | `str` | Report/path string output |
| `select_by_location` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.select_by_location(...)` | `Vector` | Vector output |
| `select_tiles_by_polygon` | `lidar` | `io_management` | `wbe.lidar.io_management.select_tiles_by_polygon(...)` | `str` | Report/path string output |
| `service_area_planning_and_coverage_optimization` | `vector` | `network_analysis` | `wbe.vector.network_analysis.service_area_planning_and_coverage_optimization(...)` | `Any` | See tool docs |
| `set_nodata_value` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.set_nodata_value(...)` | `Raster` | Raster output |
| `shadow_animation` | `terrain` | `visibility` | `wbe.terrain.visibility.shadow_animation(...)` | `tuple[str, str]` | Multiple outputs (tuple) |
| `shadow_image` | `terrain` | `visibility` | `wbe.terrain.visibility.shadow_image(...)` | `Raster` | Raster output |
| `shape_complexity_index_raster` | `raster` | `general` | `wbe.raster.shape_complexity_index_raster(...)` | `Raster` | Raster output |
| `shape_complexity_index_vector` | `vector` | `shape_metrics` | `wbe.vector.shape_metrics.shape_complexity_index_vector(...)` | `Vector` | Vector output |
| `shape_index` | `terrain` | `derivatives` | `wbe.terrain.derivatives.shape_index(...)` | `Raster` | Raster output |
| `shortest_path_network` | `vector` | `network_analysis` | `wbe.vector.network_analysis.shortest_path_network(...)` | `Vector` | Vector output |
| `shreve_stream_magnitude` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.shreve_stream_magnitude(...)` | `Raster` | Raster output |
| `sidewalk_vegetation_accessibility_monitoring` | `lidar` | `workflow_products` | `wbe.lidar.workflow_products.sidewalk_vegetation_accessibility_monitoring(...)` | `Any` | See tool docs |
| `sieve` | `raster` | `general` | `wbe.raster.sieve(...)` | `Raster` | Raster output |
| `sigmoidal_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.sigmoidal_contrast_stretch(...)` | `Raster` | Raster output |
| `simplify_features` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.simplify_features(...)` | `Vector` | Vector output |
| `sin` | `raster` | `general` | `wbe.raster.sin(...)` | `Raster` | Raster output |
| `singlepart_to_multipart` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.singlepart_to_multipart(...)` | `Vector` | Vector output |
| `sinh` | `raster` | `general` | `wbe.raster.sinh(...)` | `Raster` | Raster output |
| `sink` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.sink(...)` | `Raster` | Raster output |
| `sky_view_factor` | `terrain` | `visibility` | `wbe.terrain.visibility.sky_view_factor(...)` | `Raster` | Raster output |
| `skyline_analysis` | `terrain` | `visibility` | `wbe.terrain.visibility.skyline_analysis(...)` | `tuple[Vector, str]` | Multiple outputs (tuple) |
| `slope` | `terrain` | `derivatives` | `wbe.terrain.derivatives.slope(...)` | `Raster` | Raster output |
| `slope_vs_aspect_plot` | `terrain` | `general` | `wbe.terrain.slope_vs_aspect_plot(...)` | `str` | Report/path string output |
| `slope_vs_elev_plot` | `terrain` | `general` | `wbe.terrain.slope_vs_elev_plot(...)` | `str` | Report/path string output |
| `smooth_vectors` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.smooth_vectors(...)` | `Vector` | Vector output |
| `smooth_vegetation_residual` | `terrain` | `general` | `wbe.terrain.smooth_vegetation_residual(...)` | `Raster` | Raster output |
| `snap_endnodes` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.snap_endnodes(...)` | `Vector` | Vector output |
| `snap_events_to_routes` | `vector` | `linear_referencing` | `wbe.vector.linear_referencing.snap_events_to_routes(...)` | `Any` | See tool docs |
| `snap_points_to_network` | `vector` | `network_analysis` | `wbe.vector.network_analysis.snap_points_to_network(...)` | `Any` | See tool docs |
| `snap_pour_points` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.snap_pour_points(...)` | `Vector` | Vector output |
| `sobel_filter` | `remote_sensing` | `edge_feature_detection` | `wbe.remote_sensing.edge_feature_detection.sobel_filter(...)` | `Raster` | Raster output |
| `soil_landscape_classification` | `precision_agriculture` | `general` | `wbe.precision_agriculture.soil_landscape_classification(...)` | `tuple[Raster, Raster, Vector, str]` | Multiple outputs (tuple) |
| `solar_site_suitability_analysis` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.solar_site_suitability_analysis(...)` | `Any` | See tool docs |
| `sort_lidar` | `lidar` | `io_management` | `wbe.lidar.io_management.sort_lidar(...)` | `Lidar` | LiDAR output |
| `spatial_join` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.spatial_join(...)` | `Vector` | Vector output |
| `spectral_angle_mapper` | `remote_sensing` | `spectral_analytics` | `wbe.remote_sensing.spectral_analytics.spectral_angle_mapper(...)` | `Any` | See tool docs |
| `spectral_library_matching` | `remote_sensing` | `spectral_analytics` | `wbe.remote_sensing.spectral_analytics.spectral_library_matching(...)` | `Any` | See tool docs |
| `spherical_std_dev_of_normals` | `terrain` | `roughness_texture` | `wbe.terrain.roughness_texture.spherical_std_dev_of_normals(...)` | `Raster` | Raster output |
| `split_colour_composite` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.split_colour_composite(...)` | `tuple[Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `split_lidar` | `lidar` | `io_management` | `wbe.lidar.io_management.split_lidar(...)` | `Lidar` | LiDAR output |
| `split_lines_at_intersections` | `vector` | `network_analysis` | `wbe.vector.network_analysis.split_lines_at_intersections(...)` | `Any` | See tool docs |
| `split_vector_lines` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.split_vector_lines(...)` | `Vector` | Vector output |
| `split_with_lines` | `vector` | `geometry_processing` | `wbe.vector.geometry_processing.split_with_lines(...)` | `Vector` | Vector output |
| `sqrt` | `raster` | `general` | `wbe.raster.sqrt(...)` | `Raster` | Raster output |
| `square` | `raster` | `general` | `wbe.raster.square(...)` | `Raster` | Raster output |
| `standard_deviation_contrast_stretch` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.standard_deviation_contrast_stretch(...)` | `Raster` | Raster output |
| `standard_deviation_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.standard_deviation_filter(...)` | `Raster` | Raster output |
| `standard_deviation_of_slope` | `terrain` | `roughness_texture` | `wbe.terrain.roughness_texture.standard_deviation_of_slope(...)` | `Raster` | Raster output |
| `standard_deviation_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.standard_deviation_overlay(...)` | `Raster` | Raster output |
| `stochastic_depression_analysis` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.stochastic_depression_analysis(...)` | `Raster` | Raster output |
| `strahler_order_basins` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.strahler_order_basins(...)` | `Raster` | Raster output |
| `strahler_stream_order` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.strahler_stream_order(...)` | `Raster` | Raster output |
| `stream_link_class` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.stream_link_class(...)` | `Raster` | Raster output |
| `stream_link_identifier` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.stream_link_identifier(...)` | `Raster` | Raster output |
| `stream_link_length` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.stream_link_length(...)` | `Raster` | Raster output |
| `stream_link_slope` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.stream_link_slope(...)` | `Raster` | Raster output |
| `stream_slope_continuous` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.stream_slope_continuous(...)` | `Raster` | Raster output |
| `subbasins` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.subbasins(...)` | `Raster` | Raster output |
| `subtract` | `raster` | `overlay_math` | `wbe.raster.overlay_math.subtract(...)` | `Raster` | Raster output |
| `sum_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.sum_overlay(...)` | `Raster` | Raster output |
| `surface_area_ratio` | `terrain` | `general` | `wbe.terrain.surface_area_ratio(...)` | `Raster` | Raster output |
| `svm_classification` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.svm_classification(...)` | `Raster` | Raster output |
| `svm_regression` | `remote_sensing` | `classification` | `wbe.remote_sensing.classification.svm_regression(...)` | `Raster` | Raster output |
| `symmetrical_difference` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.symmetrical_difference(...)` | `Vector` | Vector output |
| `tan` | `raster` | `general` | `wbe.raster.tan(...)` | `Raster` | Raster output |
| `tangential_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.tangential_curvature(...)` | `Raster` | Raster output |
| `tanh` | `raster` | `general` | `wbe.raster.tanh(...)` | `Raster` | Raster output |
| `terrain_constraint_and_conflict_analysis` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.terrain_constraint_and_conflict_analysis(...)` | `Any` | See tool docs |
| `terrain_constructability_and_cost_analysis` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.terrain_constructability_and_cost_analysis(...)` | `Any` | See tool docs |
| `terrain_corrected_optical_analytics` | `remote_sensing` | `radiometric_correction` | `wbe.remote_sensing.radiometric_correction.terrain_corrected_optical_analytics(...)` | `Any` | See tool docs |
| `thicken_raster_line` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.thicken_raster_line(...)` | `Raster` | Raster output |
| `time_in_daylight` | `terrain` | `visibility` | `wbe.terrain.visibility.time_in_daylight(...)` | `Raster` | Raster output |
| `time_series_change_intelligence` | `remote_sensing` | `change_detection` | `wbe.remote_sensing.change_detection.time_series_change_intelligence(...)` | `tuple[Raster, Raster, Raster, Raster, str]` | Multiple outputs (tuple) |
| `tin_interpolation` | `raster` | `general` | `wbe.raster.tin_interpolation(...)` | `Raster` | Raster output |
| `to_degrees` | `raster` | `general` | `wbe.raster.to_degrees(...)` | `Raster` | Raster output |
| `to_radians` | `raster` | `general` | `wbe.raster.to_radians(...)` | `Raster` | Raster output |
| `tophat_transform` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.tophat_transform(...)` | `Raster` | Raster output |
| `topo_render` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.topo_render(...)` | `Raster` | Raster output |
| `topographic_hachures` | `terrain` | `general` | `wbe.terrain.topographic_hachures(...)` | `Vector` | Vector output |
| `topographic_position_animation` | `terrain` | `multiscale_signatures` | `wbe.terrain.multiscale_signatures.topographic_position_animation(...)` | `tuple[str, str]` | Multiple outputs (tuple) |
| `topological_breach_burn` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.topological_breach_burn(...)` | `tuple[Raster, Raster, Raster, Raster]` | Multiple outputs (tuple) |
| `topological_stream_order` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.topological_stream_order(...)` | `Raster` | Raster output |
| `topology_rule_autofix` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.topology_rule_autofix(...)` | `Any` | See tool docs |
| `topology_rule_validate` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.topology_rule_validate(...)` | `Any` | See tool docs |
| `topology_validation_report` | `conversion` | `geometry_topology` | `wbe.conversion.geometry_topology.topology_validation_report(...)` | `str` | Report/path string output |
| `total_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.total_curvature(...)` | `Raster` | Raster output |
| `total_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.total_filter(...)` | `Raster` | Raster output |
| `trace_downslope_flowpaths` | `hydrology` | `flow_routing` | `wbe.hydrology.flow_routing.trace_downslope_flowpaths(...)` | `Raster` | Raster output |
| `transfer_attributes` | `vector` | `network_analysis` | `wbe.vector.network_analysis.transfer_attributes(...)` | `Any` | See tool docs |
| `travelling_salesman_problem` | `vector` | `network_analysis` | `wbe.vector.network_analysis.travelling_salesman_problem(...)` | `Vector` | Vector output |
| `trend_surface` | `raster` | `general` | `wbe.raster.trend_surface(...)` | `Raster` | Raster output |
| `trend_surface_vector_points` | `raster` | `general` | `wbe.raster.trend_surface_vector_points(...)` | `Raster` | Raster output |
| `tributary_identifier` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.tributary_identifier(...)` | `Raster` | Raster output |
| `true_colour_composite` | `remote_sensing` | `enhancement_contrast` | `wbe.remote_sensing.enhancement_contrast.true_colour_composite(...)` | `Any` | See tool docs |
| `truncate` | `raster` | `general` | `wbe.raster.truncate(...)` | `Raster` | Raster output |
| `turning_bands_simulation` | `raster` | `general` | `wbe.raster.turning_bands_simulation(...)` | `Raster` | Raster output |
| `two_sample_ks_test` | `raster` | `general` | `wbe.raster.two_sample_ks_test(...)` | `str` | Report/path string output |
| `union` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.union(...)` | `Vector` | Vector output |
| `unnest_basins` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.unnest_basins(...)` | `list[Raster]` | Raster output |
| `unsharp_masking` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.unsharp_masking(...)` | `Raster` | Raster output |
| `unsphericity` | `terrain` | `derivatives` | `wbe.terrain.derivatives.unsphericity(...)` | `Raster` | Raster output |
| `update` | `vector` | `overlay_analysis` | `wbe.vector.overlay_analysis.update(...)` | `Vector` | Vector output |
| `update_nodata_cells` | `raster` | `overlay_math` | `wbe.raster.overlay_math.update_nodata_cells(...)` | `Raster` | Raster output |
| `upslope_depression_storage` | `hydrology` | `depressions_storage` | `wbe.hydrology.depressions_storage.upslope_depression_storage(...)` | `Raster` | Raster output |
| `urban_expansion_impact_assessment` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.urban_expansion_impact_assessment(...)` | `Any` | See tool docs |
| `user_defined_weights_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.user_defined_weights_filter(...)` | `Raster` | Raster output |
| `utility_corridor_encroachment_and_access_planning` | `vector` | `workflow_products` | `wbe.vector.workflow_products.utility_corridor_encroachment_and_access_planning(...)` | `Any` | See tool docs |
| `utility_corridor_encroachment_intelligence` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.utility_corridor_encroachment_intelligence(...)` | `Any` | See tool docs |
| `vector_hex_binning` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.vector_hex_binning(...)` | `Vector` | Vector output |
| `vector_lines_to_raster` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.vector_lines_to_raster(...)` | `Raster` | Raster output |
| `vector_points_to_raster` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.vector_points_to_raster(...)` | `Raster` | Raster output |
| `vector_polygons_to_raster` | `conversion` | `raster_vector_conversion` | `wbe.conversion.raster_vector_conversion.vector_polygons_to_raster(...)` | `Raster` | Raster output |
| `vector_stream_network_analysis` | `streams` | `ordering_metrics` | `wbe.streams.ordering_metrics.vector_stream_network_analysis(...)` | `Vector` | Vector output |
| `vector_summary_statistics` | `conversion` | `vector_table_io` | `wbe.conversion.vector_table_io.vector_summary_statistics(...)` | `str` | Report/path string output |
| `vehicle_routing_cvrp` | `vector` | `network_analysis` | `wbe.vector.network_analysis.vehicle_routing_cvrp(...)` | `Any` | See tool docs |
| `vehicle_routing_pickup_delivery` | `vector` | `network_analysis` | `wbe.vector.network_analysis.vehicle_routing_pickup_delivery(...)` | `Any` | See tool docs |
| `vehicle_routing_vrptw` | `vector` | `network_analysis` | `wbe.vector.network_analysis.vehicle_routing_vrptw(...)` | `Any` | See tool docs |
| `vertical_excess_curvature` | `terrain` | `derivatives` | `wbe.terrain.derivatives.vertical_excess_curvature(...)` | `Raster` | Raster output |
| `viewshed` | `terrain` | `visibility` | `wbe.terrain.visibility.viewshed(...)` | `Raster` | Raster output |
| `visibility_index` | `terrain` | `visibility` | `wbe.terrain.visibility.visibility_index(...)` | `Raster` | Raster output |
| `voronoi_diagram` | `vector` | `sampling_gridding` | `wbe.vector.sampling_gridding.voronoi_diagram(...)` | `Vector` | Vector output |
| `watershed` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.watershed(...)` | `Raster` | Raster output |
| `watershed_from_raster_pour_points` | `hydrology` | `watersheds_basins` | `wbe.hydrology.watersheds_basins.watershed_from_raster_pour_points(...)` | `Raster` | Raster output |
| `weighted_overlay` | `raster` | `overlay_math` | `wbe.raster.overlay_math.weighted_overlay(...)` | `Raster` | Raster output |
| `weighted_sum` | `raster` | `overlay_math` | `wbe.raster.overlay_math.weighted_sum(...)` | `Raster` | Raster output |
| `wetland_hydrogeomorphic_classification` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.wetland_hydrogeomorphic_classification(...)` | `Any` | See tool docs |
| `wetness_index` | `hydrology` | `hydrologic_indices` | `wbe.hydrology.hydrologic_indices.wetness_index(...)` | `Raster` | Raster output |
| `wiener_filter` | `remote_sensing` | `filters` | `wbe.remote_sensing.filters.wiener_filter(...)` | `Raster` | Raster output |
| `wilcoxon_signed_rank_test` | `raster` | `general` | `wbe.raster.wilcoxon_signed_rank_test(...)` | `str` | Report/path string output |
| `wildfire_fuel_loading_and_risk_matrix` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.wildfire_fuel_loading_and_risk_matrix(...)` | `Any` | See tool docs |
| `wind_turbine_siting` | `terrain` | `workflow_products` | `wbe.terrain.workflow_products.wind_turbine_siting(...)` | `Any` | See tool docs |
| `wisart_iterative_clustering` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.wisart_iterative_clustering(...)` | `Raster` | Raster output |
| `write_function_memory_insertion` | `remote_sensing` | `change_detection` | `wbe.remote_sensing.change_detection.write_function_memory_insertion(...)` | `Raster` | Raster output |
| `yamaguchi_4component_decomposition` | `remote_sensing` | `sar` | `wbe.remote_sensing.sar.yamaguchi_4component_decomposition(...)` | `Any` | See tool docs |
| `yield_data_conditioning_and_qa` | `precision_agriculture` | `general` | `wbe.precision_agriculture.yield_data_conditioning_and_qa(...)` | `Any` | See tool docs |
| `z_scores` | `raster` | `general` | `wbe.raster.z_scores(...)` | `Raster` | Raster output |
| `zonal_statistics` | `raster` | `general` | `wbe.raster.zonal_statistics(...)` | `Raster` | Raster output |
