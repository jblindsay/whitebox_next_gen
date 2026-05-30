# Typed Schema Rollout Matrix

## Scope Summary

This matrix tracks migration from fallback metadata (`io_role`/`data_kind` +
heuristics) to canonical explicit per-parameter `schema` metadata.

Current inventory sizing:

1. `wbtools_oss` registered tools: 702
2. `wbtools_pro` registered tools: 41

Current migration status:

1. Foundation complete in `wbcore` (schema model + serializer)
2. Frontend consumer complete in QGIS (schema-first with fallbacks)
3. Pilot tools complete:
   - `extract_streams`
   - `vector_stream_network_analysis`
4. Wave-1 batch 1 complete in OSS:
   - `flow_algorithms`: all 12 registered tools now emit explicit schema
   - `hydrology`: all 47 registered tools now emit explicit schema at runtime
   - `gis` (high-traffic batch): 13 tools now emit explicit schema at runtime
     (`buffer_vector`, `clip`, `erase`, `intersect`, `union`, `dissolve`,
     `reproject_vector`, `near`, `select_by_location`, `spatial_join`,
     `field_calculator`, `points_along_lines`, `line_polygon_clip`)
5. Python and R bindings now use unified OSS schema lookup across
   `stream_network_analysis` + `flow_algorithms` + `gis` + `hydrology`.
6. Wave-2 GIS explicit-schema batch started (May 29, 2026):
   - Added explicit schema maps for:
     `update_nodata_cells`, `buffer_raster`, `clump`,
     `euclidean_distance`, `euclidean_allocation`, `cost_distance`,
     `cost_allocation`, `cost_pathway`, `weighted_sum`,
     `weighted_overlay`, `aggregate_raster`, `reclass`,
     `reclass_equal_interval`.
7. Wave-2 GIS explicit-schema batch 2 complete (May 29, 2026):
    - Added explicit schema maps for:
       `difference`, `symmetrical_difference`, `identity`, `update`,
       `line_intersections`, `merge_line_segments`, `split_with_lines`,
       `split_lines_at_intersections`, `extract_nodes`, `centroid_vector`,
       `representative_point_vector`, `filter_vector_features_by_area`,
       `minimum_convex_hull`, `minimum_bounding_box`,
       `minimum_bounding_circle`, `minimum_bounding_envelope`.
8. Wave-2 GIS explicit-schema batch 3 complete (May 29, 2026):
   - Added explicit schema maps for:
     `add_field`, `delete_field`, `add_geometry_attributes`,
     `add_point_coordinates_to_table`, `clean_vector`, `compactness_ratio`,
     `concave_hull`, `construct_vector_tin`, `csv_points_to_vector`,
     `densify_features`, `deviation_from_regional_direction`,
     `eliminate_coincident_points`, `elongation_ratio`,
     `extend_vector_lines`, `extract_by_attribute`,
     `extract_raster_values_at_points`.
9. Wave-2 Raster explicit-schema batch 1 complete (May 29, 2026):
    - Added explicit schema map coverage for raster operator families:
       unary operators: `abs`, `arccos`, `arcosh`, `arcsin`, `arctan`,
       `arsinh`, `artanh`, `bool_not`, `ceil`, `cos`, `cosh`, `decrement`,
       `exp`, `exp2`, `floor`, `increment`, `is_nodata`, `ln`, `log10`,
       `log2`, `negate`, `reciprocal`, `round`, `sin`, `sinh`, `sqrt`,
       `square`, `tan`, `tanh`, `to_degrees`, `to_radians`, `truncate`.
       binary operators: `add`, `atan2`, `bool_and`, `bool_or`, `bool_xor`,
       `divide`, `equal_to`, `greater_than`, `greater_than_or_equal_to`,
       `integer_division`, `less_than`, `less_than_or_equal_to`, `modulo`,
       `multiply`, `not_equal_to`, `power`, `subtract`.
10. Wave-2 data_tools explicit-schema batch 1 complete (May 29, 2026):
      - Added explicit schema maps for:
         `convert_nodata_to_zero`, `modify_nodata_value`, `set_nodata_value`,
         `remove_raster_polygon_holes`, `raster_to_vector_points`,
         `raster_to_vector_lines`, `raster_to_vector_polygons`,
         `vector_points_to_raster`, `vector_lines_to_raster`,
         `vector_polygons_to_raster`, `fix_dangling_arcs`, `lines_to_polygons`,
         `polygons_to_lines`, `remove_polygon_holes`, `merge_vectors`,
         `multipart_to_singlepart`, `singlepart_to_multipart`,
         `reinitialize_attribute_table`, `export_table_to_csv`,
         `print_geotiff_tags`, `topology_validation_report`.
11. Wave-3 remote_sensing explicit-schema batch 1 complete (May 29, 2026):
      - Added explicit schema maps for 38 high-use filter/contrast tools:
         `corner_detection`, `integral_image_transform`, `line_thinning`,
         `otsu_thresholding`, `adaptive_filter`, `closing`,
         `conservative_smoothing_filter`, `diversity_filter`,
         `high_pass_filter`, `majority_filter`, `maximum_filter`,
         `mean_filter`, `minimum_filter`, `olympic_filter`, `opening`,
         `range_filter`, `refined_lee_filter`, `high_pass_median_filter`,
         `median_filter`, `percentile_filter`, `fast_almost_gaussian_filter`,
         `laplacian_of_gaussians_filter`, `gaussian_contrast_stretch`,
         `histogram_equalization`, `prewitt_filter`, `roberts_cross_filter`,
         `remove_spurs`, `flip_image`, `gamma_correction`,
         `edge_preserving_mean_filter`, `emboss_filter`, `laplacian_filter`,
         `line_detection_filter`, `gamma_map_filter`, `kuan_filter`,
         `kuwahara_filter`, `guided_filter`, `frost_filter`.
12. Wave-3 remote_sensing explicit-schema batch 2 complete (May 29, 2026):
      - Added explicit schema maps for 22 additional tools:
         `anisotropic_diffusion_filter`, `balance_contrast_enhancement`,
         `diff_of_gaussians_filter`, `direct_decorrelation_stretch`,
         `frangi_filter`, `gabor_filter_bank`, `non_local_means_filter`,
         `k_nearest_mean_filter`, `lee_filter`, `enhanced_lee_filter`,
         `histogram_matching`, `histogram_matching_two_images`,
         `min_max_contrast_stretch`, `normalized_difference_index`,
         `percentage_contrast_stretch`, `piecewise_contrast_stretch`,
         `savitzky_golay_2d_filter`, `scharr_filter`, `mosaic`,
         `mosaic_with_feathering`, `resample`, `canny_edge_detection`.
13. Wave-3 remote_sensing explicit-schema batch 3 complete (May 29, 2026):
   - Added explicit schema maps for 21 OBIA/segmentation tools:
      `build_object_hierarchy_multiscale`,
      `object_features_context_neighbors`, `object_features_shape_basic`,
      `object_features_spectral_basic`,
      `object_features_texture_glcm_basic`,
      `object_features_topology_relations`,
      `object_class_probability_maps`, `objects_enforce_min_mapping_unit`,
      `objects_boundary_refinement_pro`,
      `evaluate_segmentation_quality_pro`, `segment_graph_felzenszwalb`,
      `segment_slic_superpixels`, `segment_watershed_markers`,
      `segments_merge_small_regions`,
      `segment_multiresolution_hierarchical`,
      `segment_scale_parameter_optimizer`, `polygons_to_segments`,
      `segments_to_polygons`, `obia_pipeline_basic`,
      `propagate_labels_across_hierarchy`,
      `evaluate_object_classification_accuracy`.
14. Wave-3 remote_sensing explicit-schema batch 4 complete (May 29, 2026):
   - Added explicit schema maps for 15 classification/change tools:
      `knn_classification`, `knn_regression`,
      `fuzzy_knn_classification`, `nnd_classification`,
      `logistic_regression`, `min_dist_classification`,
      `parallelepiped_classification`,
      `random_forest_classification`, `random_forest_regression`,
      `k_means_clustering`, `modified_k_means_clustering`,
      `change_vector_analysis`,
      `image_difference_change_detection`,
      `pca_based_change_detection`, `post_classification_change`.
15. Wave-3 remote_sensing explicit-schema batch 5 complete (May 29, 2026):
   - Added explicit schema maps for 23 additional tools:
      `classify_objects_ensemble_pro`,
      `classify_objects_random_forest`,
      `classify_objects_rules_basic`,
      `classify_objects_rules_hierarchical`, `classify_objects_svm`,
      `obia_batch_orchestrator_pro`, `obia_audit_report_pro`,
      `object_uncertainty_diagnostics_pro`,
      `segments_split_low_cohesion`, `split_colour_composite`,
      `ihs_to_rgb`, `rgb_to_ihs`, `sigmoidal_contrast_stretch`,
      `standard_deviation_contrast_stretch`, `sobel_filter`,
      `standard_deviation_filter`, `total_filter`,
      `thicken_raster_line`, `tophat_transform`, `unsharp_masking`,
      `user_defined_weights_filter`, `wiener_filter`,
      `write_function_memory_insertion`.
16. Wave-3 remote_sensing explicit-schema batch 6 complete (May 29, 2026):
   - Added explicit schema maps for 14 radiometric/thermal/georeferencing tools:
      `brdf_normalization`, `terrain_corrected_optical_analytics`,
      `orthorectification`, `georeference_raster_from_control_points`,
      `land_surface_temperature_single_channel`,
      `land_surface_temperature_split_window`,
      `linear_spectral_unmixing`, `continuum_removal`,
      `dn_to_toa_reflectance`, `dark_object_subtraction`,
      `correct_vignetting`, `cloude_pottier_decomposition`,
      `freeman_durden_decomposition`, `h_alpha_wisart_classification`.
17. Wave-3 remote_sensing explicit-schema batch 7 complete (May 29, 2026):
   - Added explicit schema maps for 8 visualization/texture helper tools:
      `create_colour_composite`, `glcm_texture`, `image_slider`,
      `image_stack_profile`, `evaluate_training_sites`,
      `generalize_classified_raster`, `generalize_with_similarity`,
      `image_segmentation`.
18. Wave-3 remote_sensing explicit-schema batch 8 complete (May 29, 2026):
   - Added explicit schema maps for 10 spectral/polarimetric tools:
      `bilateral_filter`, `high_pass_bilateral_filter`,
      `gaussian_filter`, `panchromatic_sharpening`,
      `minimum_noise_fraction`, `ndvi_based_emissivity`,
      `spectral_angle_mapper`, `spectral_library_matching`,
      `yamaguchi_4component_decomposition`,
      `wisart_iterative_clustering`.
19. Wave-3 remote_sensing explicit-schema batch 9 complete (May 29, 2026):
   - Added explicit schema maps for 6 model-fit/predict tools:
      `random_forest_classification_fit`,
      `random_forest_classification_predict`,
      `random_forest_regression_fit`,
      `random_forest_regression_predict`,
      `svm_classification`, `svm_regression`.
20. Wave-3 geomorphometry explicit-schema batch 1 complete (May 29, 2026):
   - Added first module resolver and central-chain wiring.
   - Added explicit schema maps for:
      `slope`, `ruggedness_index`, `surface_area_ratio`,
      `wetness_index`, `elev_relative_to_min_max`.
21. Wave-3 geomorphometry explicit-schema batch 2 complete (May 29, 2026):
   - Added explicit schema maps for curvature family tools:
      `plan_curvature`, `profile_curvature`,
      `tangential_curvature`, `total_curvature`, `mean_curvature`,
      `gaussian_curvature`, `minimal_curvature`, `maximal_curvature`,
      `shape_index`, `curvedness`, `difference_curvature`,
      `accumulation_curvature`.
22. Wave-3 geomorphometry explicit-schema batch 3 complete (May 29, 2026):
   - Added explicit schema maps for hydrologic index tools:
      `relative_stream_power_index`, `sediment_transport_index`.
23. Wave-3 geomorphometry explicit-schema batch 4 complete (May 29, 2026):
   - Added explicit schema maps for terrain-window/multiscale tools:
      `difference_from_mean_elevation`,
      `deviation_from_mean_elevation`,
      `max_difference_from_mean`, `max_anisotropy_dev`,
      `max_elevation_deviation`, `multiscale_elevation_percentile`,
      `multiscale_roughness`, `multiscale_std_dev_normals`,
      `multiscale_roughness_signature`,
      `multiscale_std_dev_normals_signature`,
      `max_elev_dev_signature`, `max_anisotropy_dev_signature`,
      `local_hypsometric_analysis`, `multiscale_curvatures`,
      `multiscale_topographic_position_image`,
      `standard_deviation_of_slope`.
24. Wave-3 geomorphometry explicit-schema batch 5 complete (May 29, 2026):
   - Added explicit schema maps for terrain-analysis/vector-output tools:
      `breakline_mapping`, `horizon_angle`,
      `hypsometrically_tinted_hillshade`, `map_off_terrain_objects`,
      `remove_off_terrain_objects`, `smooth_vegetation_residual`,
      `low_points_on_headwater_divides`,
      `multiscale_topographic_position_class`.
25. Wave-3 geomorphometry explicit-schema batch 6 complete (May 29, 2026):
   - Added explicit schema maps for:
      `elev_relative_to_watershed_min_max`,
      `feature_preserving_smoothing_multiscale`,
      `topo_render`.
26. Wave-3 geomorphometry explicit-schema batch 7 complete (May 29, 2026):
   - Added explicit schema maps for:
      `aspect`, `convergence_index`, `openness`.
27. Wave-3 geomorphometry explicit-schema batch 8 complete (May 29, 2026):
   - Added explicit schema maps for:
      `percent_elev_range`, `relative_topographic_position`,
      `num_downslope_neighbours`, `num_upslope_neighbours`,
      `downslope_index`.
28. Wave-3 geomorphometry explicit-schema batch 9 complete (May 29, 2026):
   - Added explicit schema maps for:
      `max_downslope_elev_change`,
      `max_upslope_elev_change`,
      `min_downslope_elev_change`.
29. Wave-3 geomorphometry explicit-schema batch 10 complete (May 29, 2026):
   - Added explicit schema maps for:
      `elevation_percentile`, `max_branch_length`,
      `elev_above_pit`.
30. Wave-3 geomorphometry explicit-schema batch 11 complete (May 29, 2026):
   - Added explicit schema map for:
      `elev_above_pit_dist`.
31. Wave-3 geomorphometry explicit-schema batch 12 complete (May 29, 2026):
   - Added explicit schema maps for sky-visibility tools:
      `sky_view_factor`, `average_horizon_distance`,
      `visibility_index`, `time_in_daylight`,
      `shadow_image`, `skyline_analysis`.
32. Wave-3 geomorphometry explicit-schema batch 13 complete (May 29, 2026):
   - Added explicit schema maps for:
      `circular_variance_of_aspect`, `find_ridges`,
      `directional_relief`, `exposure_towards_wind_flux`,
      `relative_aspect`, `feature_preserving_smoothing`.
   - Note: `feature_preserving_smoothing_poisson` schema map was added in resolver
     for forward coverage, but this tool ID is not currently runtime-registered.
33. Wave-3 geomorphometry explicit-schema batch 14 complete (May 29, 2026):
   - Added explicit schema maps for:
      `hillshade`, `multidirectional_hillshade`, `viewshed`,
      `slope_vs_aspect_plot`, `slope_vs_elev_plot`, `fetch_analysis`,
      `dem_void_filling`, `horizon_area`, `shadow_animation`.
34. Wave-3 geomorphometry explicit-schema batch 15 complete (May 29, 2026):
   - Added explicit schema maps for:
      `contours_from_raster`, `contours_from_points`,
      `topographic_hachures`, `hypsometric_analysis`, `profile`,
      `geomorphons`, `multiscale_elevated_index`,
      `multiscale_low_lying_index`, `topographic_position_animation`,
      `fill_missing_data`.
   - Follow-up note: `fill_missing_data` was applied and revalidated in the
     same cycle after initial batch validation.
35. Wave-3 geomorphometry explicit-schema batch 16 complete (May 29, 2026):
   - Added explicit schema maps for:
      `assess_route`, `average_normal_vector_angular_deviation`,
      `edge_density`, `embankment_mapping`,
      `pennock_landform_classification`,
      `spherical_std_dev_of_normals`.
36. Wave-4 stream_network_analysis explicit-schema batch 1 complete (May 29, 2026):
   - Added explicit schema maps for:
      `strahler_stream_order`, `horton_stream_order`,
      `hack_stream_order`, `shreve_stream_magnitude`,
      `burn_streams`, `horton_ratios`,
      `prune_vector_streams`, `river_centerlines`,
      `ridge_and_valley_vectors`.
   - Stream module status: runtime stream-network-analysis tool IDs are now fully
     covered by explicit stream resolver mappings.
37. Wave-4 data_tools + hydrology explicit-schema batch 1 complete (May 29, 2026):
   - Added explicit schema maps for data tools:
      `add_point_coordinates_to_table`, `clean_vector`,
      `csv_points_to_vector`, `join_tables`, `merge_table_with_csv`,
      `new_raster_from_base_raster`, `new_raster_from_base_vector`,
      `topology_rule_validate`, `topology_rule_autofix`.
   - Added explicit schema map for hydrology:
      `find_noflow_cells`.
   - Module status:
      data_tools runtime tool IDs now fully covered by explicit data_tools resolver mappings;
      hydrology runtime tool IDs now fully covered by explicit hydrology resolver mappings.
38. Wave-4 raster explicit-schema batch 1 complete (May 29, 2026):
   - Added explicit schema maps for 14 raster statistics/simulation tools:
      `inplace_add`, `inplace_subtract`, `inplace_multiply`, `inplace_divide`,
      `cumulative_distribution`, `random_field`, `fft_random_field`,
      `random_sample`, `attribute_histogram`, `attribute_scattergram`,
      `attribute_correlation`, `cross_tabulation`, `anova`, `crispness_index`.
   - Batch note: inplace arithmetic tools now explicitly model mixed operand mode
     using `input_existing_or_number` on `input2` with raster dataset semantics.
39. Wave-4 raster explicit-schema batch 2 complete (May 29, 2026):
   - Added explicit schema maps for 13 additional raster/vector-statistics tools:
      `raster_summary_stats`, `raster_histogram`, `list_unique_values_raster`,
      `z_scores`, `rescale_value_range`, `ks_normality_test`,
      `phi_coefficient`, `max`, `min`, `quantiles`, `list_unique_values`,
      `root_mean_square_error`, `zonal_statistics`.
40. Wave-4 raster explicit-schema batch 3 complete (May 29, 2026):
   - Added explicit schema maps for the remaining 17 runtime raster IDs:
      `conditional_evaluation`, `dbscan`, `image_autocorrelation`,
      `image_correlation`, `image_correlation_neighbourhood_analysis`,
      `image_regression`, `inverse_pca`, `kappa_index`,
      `paired_sample_t_test`, `principal_component_analysis`,
      `raster_calculator`, `trend_surface`, `trend_surface_vector_points`,
      `turning_bands_simulation`, `two_sample_ks_test`,
      `wilcoxon_signed_rank_test`, and parser-visibility fix for
      `raster_summary_stats` arm formatting in resolver counting.
   - Module status: raster runtime tool IDs are now fully covered by explicit
     raster resolver mappings (`raster_runtime_missing 0`).
41. Wave-5 GIS explicit-schema batch 1 complete (May 29, 2026):
   - Added explicit schema maps for 20 network/routing tools:
      `build_network_topology`, `generate_network_nodes`,
      `shortest_path_network`, `closest_facility_network`,
      `location_allocation_network`, `k_shortest_paths_network`,
      `network_od_cost_matrix`, `network_routes_from_od`,
      `network_service_area`, `network_centrality_metrics`,
      `network_accessibility_metrics`, `network_connected_components`,
      `network_node_degree`, `network_topology_audit`,
      `multimodal_shortest_path`, `multimodal_od_cost_matrix`,
      `multimodal_routes_from_od`, `od_sensitivity_analysis`,
      `snap_points_to_network`, `snap_events_to_routes`.
   - Module status: GIS runtime unresolved count reduced from 93 to 73.
42. Wave-5 GIS explicit-schema batch 2 complete (May 29, 2026):
   - Added explicit schema maps for 20 additional shape/patch and assignment tools:
      `boundary_shape_complexity`, `edge_proportion`, `hole_proportion`,
      `linearity_index`, `narrowness_index`, `narrowness_index_vector`,
      `patch_orientation`, `perimeter_area_ratio`, `radius_of_gyration`,
      `related_circumscribing_circle`, `shape_complexity_index_raster`,
      `shape_complexity_index_vector`, `polygon_area`, `polygon_perimeter`,
      `polygon_short_axis`, `polygon_long_axis`, `raster_area`,
      `raster_perimeter`, `map_features`, `raster_cell_assignment`.
   - Module status: GIS runtime unresolved count reduced from 73 to 53.
43. Wave-5 GIS explicit-schema batch 3 complete (May 29, 2026):
   - Added explicit schema maps for 25 interpolation/grid/raster-vector tools:
      `centroid_raster`, `clip_raster_to_polygon`, `create_plane`,
      `erase_polygon_from_raster`, `filter_raster_features_by_area`,
      `find_lowest_or_highest_points`, `find_patch_edge_cells`, `heat_map`,
      `hexagonal_grid_from_raster_base`, `hexagonal_grid_from_vector_base`,
      `idw_interpolation`, `layer_footprint_raster`, `medoid`,
      `modified_shepard_interpolation`, `natural_neighbour_interpolation`,
      `nearest_neighbour_interpolation`, `nibble`, `pick_from_list`,
      `polygonize`, `radial_basis_function_interpolation`,
      `rectangular_grid_from_raster_base`,
      `rectangular_grid_from_vector_base`, `tin_interpolation`,
      `voronoi_diagram`, `random_points_in_polygon`.
   - Module status: GIS runtime unresolved count reduced from 53 to 24
     (parser-aware grouped-arm extraction basis).
44. Wave-5 GIS explicit-schema batch 4 complete (May 29, 2026):
    - Added explicit schema maps for the remaining 24 GIS runtime IDs:
         `locate_points_along_routes`, `map_matching_v1`, `rename_field`,
         `route_calibrate`, `route_event_lines_from_layer`,
         `route_event_lines_from_table`, `route_event_merge`,
         `route_event_overlay`, `route_event_points_from_layer`,
         `route_event_points_from_table`, `route_event_split`,
         `route_measure_qa`, `route_recalibrate`, `simplify_features`,
         `smooth_vectors`, `snap_endnodes`, `split_vector_lines`,
         `transfer_attributes`, `travelling_salesman_problem`,
         `vector_hex_binning`, `vector_summary_statistics`,
         `vehicle_routing_cvrp`, `vehicle_routing_pickup_delivery`,
         `vehicle_routing_vrptw`.
    - Follow-up fidelity fix in same cycle: corrected `events` dataset kind to
       `table` for table-backed route-event tools and split CVRP/VRPTW objective
       enums to match per-tool contracts.
    - Module status: GIS runtime unresolved count reduced from 24 to 0.
45. Wave-6 LiDAR explicit-schema batch 1 complete (May 29, 2026):
    - Added first explicit schema resolver in `lidar_processing` and wired it
       into central `tool_param_schemas` chain.
    - Added explicit schema maps for 24 LiDAR tools:
         `ascii_to_las`, `las_to_ascii`, `las_to_shapefile`, `lidar_info`,
         `lidar_histogram`, `lidar_point_stats`, `lidar_point_return_analysis`,
         `lidar_hex_bin`, `lidar_tile_footprint`, `lidar_contour`,
         `lidar_construct_vector_tin`, `lidar_colourize`,
         `colourize_based_on_class`, `colourize_based_on_point_returns`,
         `lidar_join`, `lidar_shift`, `lidar_tile`, `split_lidar`, `lidar_thin`,
         `lidar_thin_high_density`, `sort_lidar`, `remove_duplicates`,
         `recover_flightline_info`, `find_flightline_edge_points`.
    - Module status: LiDAR runtime unresolved count reduced from 65 to 41.
   46. Wave-6 LiDAR explicit-schema batch 2 complete (May 29, 2026):
      - Added explicit schema maps for 40 additional LiDAR tools:
         `classify_buildings_in_lidar`, `classify_lidar`,
         `classify_overlap_points`, `clip_lidar_to_polygon`,
         `erase_polygon_from_lidar`, `filter_lidar`,
         `filter_lidar_by_percentile`, `filter_lidar_by_reference_surface`,
         `filter_lidar_classes`, `filter_lidar_noise`,
         `filter_lidar_scan_angles`, `flightline_overlap`,
         `height_above_ground`, `individual_tree_detection`,
         `individual_tree_segmentation`, `lidar_block_maximum`,
         `lidar_block_minimum`, `lidar_classify_subset`,
         `lidar_digital_surface_model`, `lidar_eigenvalue_features`,
         `lidar_elevation_slice`, `lidar_ground_point_filter`,
         `lidar_hillshade`, `lidar_idw_interpolation`, `lidar_kappa`,
         `lidar_nearest_neighbour_gridding`, `lidar_point_density`,
         `lidar_radial_basis_function_interpolation`, `lidar_ransac_planes`,
         `lidar_remove_outliers`, `lidar_rooftop_analysis`,
         `lidar_segmentation`, `lidar_segmentation_based_filter`,
         `lidar_sibson_interpolation`, `lidar_tin_gridding`,
         `lidar_tophat_transform`, `modify_lidar`, `normal_vectors`,
         `normalize_lidar`, `select_tiles_by_polygon`.
      - Module status: LiDAR runtime unresolved count reduced from 41 to 1.
   47. Wave-6 LiDAR explicit-schema batch 3 complete (May 29, 2026):
      - Added explicit schema map for remaining LiDAR submodule tool:
         `improved_ground_point_filter`.
      - Module status: LiDAR runtime unresolved count reduced from 1 to 0.
   48. Wave-7 cross-module explicit-schema closure batch complete (May 29, 2026):
         - Added explicit schema maps for final 26 runtime-unresolved IDs:
            GIS overlays/stat operators: `average_overlay`, `count_if`,
            `highest_position`, `lowest_position`, `max_absolute_overlay`,
            `max_overlay`, `min_absolute_overlay`, `min_overlay`,
            `multiply_overlay`, `percent_equal_to`, `percent_greater_than`,
            `percent_less_than`, `standard_deviation_overlay`, `sum_overlay`;
            block tools: `block_maximum`, `block_minimum`;
            GIS submodule tools: `download_osm_vector`, `sieve`;
            geomorphometry pro-curvature tools: `unsphericity`,
            `ring_curvature`, `rotor`, `horizontal_excess_curvature`,
            `vertical_excess_curvature`, `generating_function`,
            `principal_curvature_direction`, `casorati_curvature`.
         - Module status: runtime unresolved count reduced from 26 to 0
            (runtime-ID intersection basis).

Current runtime completeness snapshot (May 29, 2026):

1. OSS visible tools in current runtime: 702
2. Tools with missing `schema`: 0
3. Tools with missing parameter descriptions: 0
4. Tools leaking wrapper-only `callback` parameter into catalog metadata: 0
5. Tools with alias duplicates (e.g., `d8_pntr` + `d8_pointer`): 0
6. Status interpretation: runtime metadata completeness is achieved for OSS in
   the current build, but explicit per-module schema-map migration waves remain
   in progress as documented below.
7. Measured explicit schema-map IDs across active OSS resolvers (runtime-ID intersection basis):
   702/702 (100.0%) after raster/data_tools/remote-sensing/geomorphometry/stream/hydrology/gis/lidar acceleration batches and final cross-module closure.
   Note: previous ad hoc counting overestimated coverage by including non-runtime
   identifiers/aliases parsed from resolver source.

## Rollout Policy

1. New or modified tools must include explicit schema.
2. Existing tools migrate in batches by module/family.
3. Fallback behavior remains during migration only.
4. Add CI coverage metric: percentage of tools emitting explicit schema.

## OSS Rollout (702 Tools)

### Wave 1 (Immediate reliability)

1. `stream_network_analysis` module
2. `hydrology` module
3. `flow_algorithms` module
4. High-traffic `gis` tools used by QGIS workflows

Definition of done:

1. Every tool in these modules has explicit schema map entries.
2. QGIS widget decisions for these tools use schema path, not heuristics.
3. Snapshot metadata tests cover representative tool variants.

### Wave 2 (Vector/raster core)

1. Remaining `gis` module tools
2. `raster` module tools
3. `data_tools` module tools

Definition of done:

1. Layer/file/field/enum behavior is schema-driven.
2. Mixed input modes (file-or-number, list inputs) are explicit.

### Wave 3 (Lidar and remote sensing breadth)

1. `lidar_processing` module
2. `remote_sensing` module
3. `geomorphometry` module

Definition of done:

1. LiDAR/table/report outputs mapped via explicit schema.
2. Large option-rich tools emit enum schema explicitly.

### OSS Tracking Checklist

1. [x] Add module-level schema map helpers for all `wbtools_oss` modules.
2. [x] Require explicit schema for changed tools via CI rule.
3. [x] Add coverage report command: `schema_coverage_report`.
4. [x] Reach 25% explicit coverage.
5. [x] Reach 50% explicit coverage.
6. [x] Reach 80% explicit coverage.
7. [x] Reach 100% explicit coverage for public QGIS-facing tools.

Current measured runtime status (Wave 1 scope):

1. `stream_network_analysis`: representative tools verified (`extract_streams`,
   `vector_stream_network_analysis`)
2. `flow_algorithms`: 12/12 tools emitting schema
3. `hydrology`: 47/47 tools emitting schema
4. `gis` high-traffic batch: 13/13 tools emitting schema

Current measured runtime status (full OSS catalog):

1. 702/702 tools emitting parameter schema metadata in runtime catalog output
2. 702/702 tools emitting non-empty parameter descriptions in runtime catalog output
3. Wrapper-only parameter leakage blocked in catalog output (`callback`,
   `_path` aliases, duplicate canonical+alias pairs)
4. Wave-2 GIS batch regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
5. Wave-2 GIS batch 2 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
6. Wave-2 GIS batch 3 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
7. Wave-2 raster batch 1 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
8. Wave-2 data_tools batch 1 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
9. Wave-3 remote_sensing batch 1 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
10. Wave-3 remote_sensing batch 2 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
11. Wave-3 remote_sensing batch 3 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
12. Wave-3 remote_sensing batch 4 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
13. Wave-3 remote_sensing batch 5 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
14. Wave-3 remote_sensing batch 6 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
15. Wave-3 remote_sensing batch 7 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
16. Wave-3 remote_sensing batch 8 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
17. Wave-3 remote_sensing batch 9 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
18. Wave-3 geomorphometry batch 1 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
19. Wave-3 geomorphometry batch 2 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
20. Wave-3 geomorphometry batch 3 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
21. Wave-3 geomorphometry batch 4 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
22. Wave-3 geomorphometry batch 5 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
23. Wave-4 raster batch 1 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
24. Wave-4 raster batch 2 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
25. Wave-4 raster batch 3 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
26. Wave-5 GIS batch 1 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
27. Wave-5 GIS batch 2 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
28. Wave-5 GIS batch 3 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
29. Wave-5 GIS batch 4 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
30. Wave-6 LiDAR batch 1 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
31. Wave-6 LiDAR batch 2 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
32. Wave-6 LiDAR batch 3 regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)
33. Wave-7 cross-module closure regression check passed after explicit-map additions
   (`schema_coverage_report.py --strict`)

Important distinction:

1. The above confirms runtime metadata quality for all OSS tools.
2. As of Wave-7 cross-module closure, runtime-ID explicit schema-map migration
   is complete for OSS tools in active runtime validation.

Coverage command:

1. Runtime coverage can be reported with:
   `./.venv-wbw/bin/python scripts/schema_coverage_report.py`
2. Runtime metadata contract can be enforced with:
   `./.venv-wbw/bin/python scripts/schema_coverage_report.py --strict`
3. CI-light OSS+Pro schema contract gate can be enforced with:
   `bash scripts/schema_contract_gate.sh`

## Pro Rollout (41 Registered Tools)

`wbtools_pro` should migrate under the same contract to avoid frontend
inconsistency between OSS and Pro tool families.

Current runtime note (May 29, 2026):

1. Active validation runtime is OSS-only (`compiled_with_pro_support: false`).
2. Pro tools are not currently validated in the active runtime snapshot and
   remain tracked as pending migration items.
3. Pro migration baseline inventory captured in:
   `/Users/johnlindsay/Documents/programming/Rust/wbtools_pro/docs/PRO_TYPED_SCHEMA_MIGRATION_INVENTORY_2026-05-29.md`
   (41 registered tools with source-file mapping from `src/lib.rs` registrations).
4. Pro Wave-P1 (GIS) started:
   - Added `gis_tool_param_schemas` for first 8 registered Pro GIS tools.
   - Added central Pro resolver dispatch in `src/tools/mod.rs` and crate-level
     export in `src/lib.rs` (`tool_param_schemas`).
   - `cargo check` passed in `wbtools_pro` after wiring.
5. Pro Wave-P2 (LiDAR) continued:
    - Added `lidar_processing_tool_param_schemas` for:
       `lidar_qa_and_confidence`, `lidar_change_and_disturbance_analysis`,
       and `sidewalk_vegetation_accessibility_monitoring`.
    - Extended central Pro resolver dispatch to include LiDAR module.
    - `cargo check` passed in `wbtools_pro` after update.
6. Pro Wave-P3 (cross-module first-20 completion):
    - Added resolver coverage for:
       remote sensing: `remote_sensing_change_detection`,
       `time_series_change_intelligence`, `sar_coregistration`,
       `sar_analysis_readiness`, `sar_interferogram_coherence`;
       siting: `wind_turbine_siting`,
       `terrain_constraint_and_conflict_analysis`,
       `terrain_constructability_and_cost_analysis`;
       geomorphometry: `soil_landscape_classification`;
       lidar: `lidar_terrain_product_suite`.
    - Extended central Pro resolver dispatch for remote-sensing, siting,
       and geomorphometry modules.
    - `cargo check` passed in `wbtools_pro` after update.
7. Pro Wave-P4 (workflow-products completion):
    - Added resolver coverage for remaining workflow-products tools:
       `multi_sensor_fusion_monitoring`,
       `brdf_surface_reflectance_consistency`,
       `corridor_mapping_intelligence`,
       `solar_site_suitability_analysis`,
       `urban_expansion_impact_assessment`,
       `wetland_hydrogeomorphic_classification`,
       `landslide_susceptibility_assessment`,
       `river_corridor_health_assessment`,
       `precision_irrigation_optimization`,
       `precision_ag_yield_zone_intelligence`,
       `in_season_crop_stress_intervention_planning`,
       `field_trafficability_and_operation_planning`,
       `yield_data_conditioning_and_qa`,
       `utility_corridor_encroachment_intelligence`,
       `forestry_structure_and_biomass_intelligence`,
       `guided_uav_image_intake_workflow`,
       `registration_oriented_feature_workflow`,
       `carbon_sequestration_verification_audit`,
       `wildfire_fuel_loading_and_risk_matrix`,
       `mine_site_reclamation_compliance_tracker`.
    - Extended central Pro resolver dispatch to include workflow-products module.
    - `cargo check` passed in `wbtools_pro` after update.
8. Pro coverage guard script added and validated:
    - Added `scripts/pro_schema_coverage_report.py` in `wbtools_pro`.
    - Strict check passed with current source snapshot:
       `registered_tool_id_count 41`, `missing_resolver_id_count 0`.

### Pro Tool Checklist

1. [x] `NetworkReadinessAndDiagnosticsIntelligenceTool`
2. [x] `EmergencyScenarioRoutingAndAccessibilitySimulatorTool`
3. [x] `FleetRoutingAndDispatchOptimizerTool`
4. [x] `MarketAccessAndSiteIntelligenceWorkflowTool`
5. [x] `ParcelAndLandFabricTopologyComplianceWorkflowTool`
6. [x] `RouteEventGovernanceForLinearAssetsTool`
7. [x] `ServiceAreaPlanningAndCoverageOptimizationTool`
8. [x] `UtilityCorridorEncroachmentAndAccessPlanningTool`
9. [x] `LiDARQAAndConfidenceTool`
10. [x] `LiDARChangeAndDisturbanceAnalysisTool`
11. [x] `SidewalkVegetationAccessibilityMonitoringTool`
12. [x] `RemoteSensingChangeDetectionTool`
13. [x] `WindTurbineSitingTool`
14. [x] `TerrainConstraintAndConflictAnalysisTool`
15. [x] `TerrainConstructabilityAndCostAnalysisTool`
16. [x] `LiDARTerrainProductSuiteTool`
17. [x] `SoilLandscapeClassificationTool`
18. [x] `TimeSeriesChangeIntelligenceTool`
19. [x] `SarCoregistrationTool`
20. [x] `SarAnalysisReadyTool`
21. [x] `SarInterferogramCoherenceTool`
22. [x] `MultiSensorFusionMonitoringTool`
23. [x] `BrdfSurfaceReflectanceConsistencyTool`
24. [x] `CorridorMappingIntelligenceTool`
25. [x] `SolarSiteSuitabilityAnalysisTool`
26. [x] `UrbanExpansionImpactAssessmentTool`
27. [x] `WetlandHydrogeomorphicClassificationTool`
28. [x] `LandslideSusceptibilityAssessmentTool`
29. [x] `RiverCorridorHealthAssessmentTool`
30. [x] `PrecisionIrrigationOptimizationTool`
31. [x] `PrecisionAgYieldZoneIntelligenceTool`
32. [x] `InSeasonCropStressInterventionPlanningTool`
33. [x] `FieldTrafficabilityAndOperationPlanningTool`
34. [x] `YieldDataConditioningAndQaTool`
35. [x] `UtilityCorridorEncroachmentIntelligenceTool`
36. [x] `ForestryStructureAndBiomassIntelligenceTool`
37. [x] `GuidedUavImageIntakeWorkflowTool`
38. [x] `RegistrationOrientedFeatureWorkflowTool`
39. [x] `CarbonSequestrationVerificationAuditTool`
40. [x] `WildfireFuelLoadingAndRiskMatrixTool`
41. [x] `MineSiteReclamationComplianceTrackerTool`

### Pro Tracking Checklist

1. [x] Generate 41-tool Pro migration inventory from `src/lib.rs` registrations.
2. [x] Add module-level schema map helper equivalents in `wbtools_pro` (GIS/LiDAR/remote-sensing/siting/geomorphometry/workflow-products).
3. [x] Migrate first 10 Pro tools (network + stream-adjacent workflows).
4. [x] Migrate first 20 Pro tools (add lidar + SAR workflows).
5. [x] Migrate all 41 registered Pro tools.

Pro coverage command:

1. Static source coverage can be reported with:
   `python3 scripts/pro_schema_coverage_report.py`
2. Pro resolver contract can be enforced with:
   `python3 scripts/pro_schema_coverage_report.py --strict`


## Validation and Governance

1. Add metadata snapshot tests in Python/R bindings for pilot and migrated sets.
2. Add QGIS widget contract tests for representative schema kinds:
   raster/vector/lidar/list/enum/field/output.
3. Add release checklist gate:
   no crate release without changelog entry and updated schema coverage status.

## Exit Criteria

1. Schema-first behavior active for all QGIS-visible tools.
2. Fallback heuristics used only for explicitly allowlisted legacy wrappers.
3. OSS + Pro schema coverage reaches targeted threshold agreed for release.