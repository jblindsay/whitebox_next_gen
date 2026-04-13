# Auto-generated wbw_r wrappers
# Regenerate via generate_r_wrapper_module_with_options(include_pro, tier).

wbw_make_session <- function(floating_license_id = NULL, include_pro = NULL, tier = "open", provider_url = NULL, machine_id = NULL, customer_id = NULL) {
  resolved_include_pro <- if (is.null(include_pro)) !is.null(floating_license_id) else include_pro

  run_tool <- function(tool_id, args = list()) {
    args_json <- jsonlite::toJSON(args, auto_unbox = TRUE, null = "null")
    if (!is.null(floating_license_id)) {
      out_json <- run_tool_json_with_floating_license_id_options(
        tool_id,
        args_json,
        floating_license_id,
        resolved_include_pro,
        tier,
        provider_url,
        machine_id,
        customer_id
      )
    } else {
      out_json <- run_tool_json_with_options(tool_id, args_json, resolved_include_pro, tier)
    }
    out <- jsonlite::fromJSON(out_json, simplifyVector = FALSE)
    wbw_coerce_tool_output(out, session = session)
  }

  list_tools <- function() {
    if (!is.null(floating_license_id)) {
      out_json <- list_tools_json_with_floating_license_id_options(
        floating_license_id,
        resolved_include_pro,
        tier,
        provider_url,
        machine_id,
        customer_id
      )
    } else {
      out_json <- list_tools_json_with_options(resolved_include_pro, tier)
    }
    jsonlite::fromJSON(out_json, simplifyVector = FALSE)
  }

  session <- new.env(parent = emptyenv())
  session$run_tool <- run_tool
  session$list_tools <- list_tools
  session$abs <- function(...) {
    # Calculates the absolute value of each raster cell.
    run_tool("abs", list(...))
  }
  session$accumulation_curvature <- function(...) {
    # Calculates accumulation curvature from a DEM.
    run_tool("accumulation_curvature", list(...))
  }
  session$adaptive_filter <- function(...) {
    # Performs adaptive thresholded mean replacement based on local z-scores.
    run_tool("adaptive_filter", list(...))
  }
  session$add <- function(...) {
    # Adds two rasters on a cell-by-cell basis.
    run_tool("add", list(...))
  }
  session$add_field <- function(...) {
    # Adds a new attribute field with an optional default value.
    run_tool("add_field", list(...))
  }
  session$add_geometry_attributes <- function(...) {
    # Adds area, length, perimeter, and centroid attributes to vector features.
    run_tool("add_geometry_attributes", list(...))
  }
  session$add_point_coordinates_to_table <- function(...) {
    # Copies a point layer and appends XCOORD and YCOORD attribute fields.
    run_tool("add_point_coordinates_to_table", list(...))
  }
  session$aggregate_raster <- function(...) {
    # Reduces raster resolution by aggregating blocks using mean, sum, min, max, or range.
    run_tool("aggregate_raster", list(...))
  }
  session$anisotropic_diffusion_filter <- function(...) {
    # Performs Perona-Malik edge-preserving anisotropic diffusion smoothing.
    run_tool("anisotropic_diffusion_filter", list(...))
  }
  session$anova <- function(...) {
    # Performs one-way ANOVA on raster values grouped by class raster categories.
    run_tool("anova", list(...))
  }
  session$arccos <- function(...) {
    # Computes the inverse cosine (arccos) of each raster cell.
    run_tool("arccos", list(...))
  }
  session$arcosh <- function(...) {
    # Computes the inverse hyperbolic cosine of each raster cell.
    run_tool("arcosh", list(...))
  }
  session$arcsin <- function(...) {
    # Computes the inverse sine (arcsin) of each raster cell.
    run_tool("arcsin", list(...))
  }
  session$arctan <- function(...) {
    # Computes the inverse tangent (arctan) of each raster cell.
    run_tool("arctan", list(...))
  }
  session$arsinh <- function(...) {
    # Computes the inverse hyperbolic sine of each raster cell.
    run_tool("arsinh", list(...))
  }
  session$artanh <- function(...) {
    # Computes the inverse hyperbolic tangent of each raster cell.
    run_tool("artanh", list(...))
  }
  session$ascii_to_las <- function(...) {
    # Converts one or more ASCII LiDAR point files to LAS.
    run_tool("ascii_to_las", list(...))
  }
  session$aspect <- function(...) {
    # Calculates slope aspect in degrees clockwise from north.
    run_tool("aspect", list(...))
  }
  session$atan2 <- function(...) {
    # Computes the four-quadrant inverse tangent using two rasters on a cell-by-cell basis.
    run_tool("atan2", list(...))
  }
  session$attribute_correlation <- function(...) {
    # Performs Pearson correlation analysis on numeric vector attribute fields.
    run_tool("attribute_correlation", list(...))
  }
  session$attribute_histogram <- function(...) {
    # Creates a histogram for numeric field values in a vector attribute table.
    run_tool("attribute_histogram", list(...))
  }
  session$attribute_scattergram <- function(...) {
    # Computes scatterplot summary statistics between two numeric vector fields.
    run_tool("attribute_scattergram", list(...))
  }
  session$average_flowpath_slope <- function(...) {
    # Calculates average slope gradient of flowpaths passing through each DEM cell.
    run_tool("average_flowpath_slope", list(...))
  }
  session$average_normal_vector_angular_deviation <- function(...) {
    # Calculates local mean angular deviation between original and smoothed surface normals.
    run_tool("average_normal_vector_angular_deviation", list(...))
  }
  session$average_overlay <- function(...) {
    # Computes the per-cell average across a raster stack, ignoring NoData unless all inputs are NoData.
    run_tool("average_overlay", list(...))
  }
  session$average_upslope_flowpath_length <- function(...) {
    # Computes the average upslope flowpath length passing through each DEM cell.
    run_tool("average_upslope_flowpath_length", list(...))
  }
  session$balance_contrast_enhancement <- function(...) {
    # Reduces colour bias in a packed RGB image using per-channel parabolic stretches.
    run_tool("balance_contrast_enhancement", list(...))
  }
  session$basins <- function(...) {
    # Delineates all D8 drainage basins that drain to valid-data edges.
    run_tool("basins", list(...))
  }
  session$bilateral_filter <- function(...) {
    # Performs an edge-preserving bilateral smoothing filter on a raster image.
    run_tool("bilateral_filter", list(...))
  }
  session$block_maximum <- function(...) {
    # Rasterizes point features by assigning the maximum value observed within each output cell.
    run_tool("block_maximum", list(...))
  }
  session$block_minimum <- function(...) {
    # Rasterizes point features by assigning the minimum value observed within each output cell.
    run_tool("block_minimum", list(...))
  }
  session$bool_and <- function(...) {
    # Computes a logical AND of two rasters on a cell-by-cell basis.
    run_tool("bool_and", list(...))
  }
  session$bool_not <- function(...) {
    # Computes a logical NOT of each raster cell, outputting 1 for zero-valued cells and 0 otherwise.
    run_tool("bool_not", list(...))
  }
  session$bool_or <- function(...) {
    # Computes a logical OR of two rasters on a cell-by-cell basis.
    run_tool("bool_or", list(...))
  }
  session$bool_xor <- function(...) {
    # Computes a logical XOR of two rasters on a cell-by-cell basis.
    run_tool("bool_xor", list(...))
  }
  session$boundary_shape_complexity <- function(...) {
    # Calculates raster patch boundary-shape complexity using a line-thinned skeleton branch metric.
    run_tool("boundary_shape_complexity", list(...))
  }
  session$breach_depressions_least_cost <- function(...) {
    # Breaches depressions in a DEM using a constrained least-cost pathway search.
    run_tool("breach_depressions_least_cost", list(...))
  }
  session$breach_single_cell_pits <- function(...) {
    # Breaches single-cell pits in a DEM by carving one-cell channels.
    run_tool("breach_single_cell_pits", list(...))
  }
  session$buffer_raster <- function(...) {
    # Creates a binary buffer zone around non-zero, non-NoData raster cells within a specified distance.
    run_tool("buffer_raster", list(...))
  }
  session$buffer_vector <- function(...) {
    # Creates polygon buffers around point, line, and polygon vector geometries with configurable cap and join styles.
    run_tool("buffer_vector", list(...))
  }
  session$burn_streams <- function(...) {
    # Burns a stream network into a DEM by decreasing stream-cell elevations.
    run_tool("burn_streams", list(...))
  }
  session$burn_streams_at_roads <- function(...) {
    # Lowers stream elevations near stream-road crossings to breach road embankments in a DEM.
    run_tool("burn_streams_at_roads", list(...))
  }
  session$canny_edge_detection <- function(...) {
    # Applies Canny multi-stage edge detection (Gaussian blur → Sobel gradient → non-maximum suppression → double threshold → hysteresis).
    run_tool("canny_edge_detection", list(...))
  }
  session$casorati_curvature <- function(...) {
    # Calculates Casorati curvature from a DEM.
    run_tool("casorati_curvature", list(...))
  }
  session$ceil <- function(...) {
    # Rounds each raster cell upward to the nearest integer.
    run_tool("ceil", list(...))
  }
  session$centroid_raster <- function(...) {
    # Calculates the centroid cell for each positive-valued patch ID in a raster.
    run_tool("centroid_raster", list(...))
  }
  session$centroid_vector <- function(...) {
    # Computes centroid points from vector features.
    run_tool("centroid_vector", list(...))
  }
  session$change_vector_analysis <- function(...) {
    # Performs change vector analysis on two-date multispectral datasets and returns magnitude and direction rasters.
    run_tool("change_vector_analysis", list(...))
  }
  session$circular_variance_of_aspect <- function(...) {
    # Calculates local circular variance of aspect within a moving neighbourhood.
    run_tool("circular_variance_of_aspect", list(...))
  }
  session$classify_buildings_in_lidar <- function(...) {
    # Assigns classification 6 to LiDAR points falling inside building footprint polygons.
    run_tool("classify_buildings_in_lidar", list(...))
  }
  session$classify_lidar <- function(...) {
    # Performs LiDAR classification into ground, building, and vegetation using neighborhood geometry and segmentation.
    run_tool("classify_lidar", list(...))
  }
  session$classify_overlap_points <- function(...) {
    # Flags or filters LiDAR points in grid cells containing multiple point source IDs.
    run_tool("classify_overlap_points", list(...))
  }
  session$clean_vector <- function(...) {
    # Removes null and invalid vector geometries (e.g., undersized lines/polygons) while preserving valid features and attributes.
    run_tool("clean_vector", list(...))
  }
  session$clip <- function(...) {
    # Clips input polygons to overlay polygon boundaries using topology-based intersection.
    run_tool("clip", list(...))
  }
  session$clip_lidar_to_polygon <- function(...) {
    # Retains only LiDAR points that fall within polygon geometry.
    run_tool("clip_lidar_to_polygon", list(...))
  }
  session$clip_raster_to_polygon <- function(...) {
    # Clips a raster to polygon extents; outside polygon cells are set to NoData.
    run_tool("clip_raster_to_polygon", list(...))
  }
  session$closing <- function(...) {
    # Performs a morphological closing operation using a rectangular structuring element.
    run_tool("closing", list(...))
  }
  session$clump <- function(...) {
    # Groups contiguous equal-valued raster cells into unique patch identifiers.
    run_tool("clump", list(...))
  }
  session$colourize_based_on_class <- function(...) {
    # Sets LiDAR point RGB values based on point classifications.
    run_tool("colourize_based_on_class", list(...))
  }
  session$colourize_based_on_point_returns <- function(...) {
    # Sets LiDAR point RGB values based on return-type categories.
    run_tool("colourize_based_on_point_returns", list(...))
  }
  session$compactness_ratio <- function(...) {
    # Computes compactness ratio (perimeter of equivalent circle / actual perimeter) for polygon features.
    run_tool("compactness_ratio", list(...))
  }
  session$concave_hull <- function(...) {
    # Creates concave hull polygons around all input feature coordinates.
    run_tool("concave_hull", list(...))
  }
  session$conditional_evaluation <- function(...) {
    # Performs if-then-else conditional evaluation on raster cells.
    run_tool("conditional_evaluation", list(...))
  }
  session$conservative_smoothing_filter <- function(...) {
    # Performs conservative smoothing by clipping impulse outliers to neighborhood bounds.
    run_tool("conservative_smoothing_filter", list(...))
  }
  session$construct_vector_tin <- function(...) {
    # Constructs a triangular irregular network (TIN) from an input point set using Delaunay triangulation.
    run_tool("construct_vector_tin", list(...))
  }
  session$contours_from_points <- function(...) {
    # Creates contour polylines from point elevations using a Delaunay TIN.
    run_tool("contours_from_points", list(...))
  }
  session$contours_from_raster <- function(...) {
    # Creates contour polylines from a raster surface model.
    run_tool("contours_from_raster", list(...))
  }
  session$convergence_index <- function(...) {
    # Calculates the convergence/divergence index from local neighbour aspect alignment.
    run_tool("convergence_index", list(...))
  }
  session$convert_nodata_to_zero <- function(...) {
    # Replaces raster nodata cells with 0 while leaving valid cells unchanged.
    run_tool("convert_nodata_to_zero", list(...))
  }
  session$corner_detection <- function(...) {
    # Identifies corner patterns in binary rasters using hit-and-miss templates.
    run_tool("corner_detection", list(...))
  }
  session$correct_vignetting <- function(...) {
    # Reduces brightness fall-off away from a principal point using a cosine lens model.
    run_tool("correct_vignetting", list(...))
  }
  session$cos <- function(...) {
    # Computes the cosine of each raster cell value.
    run_tool("cos", list(...))
  }
  session$cosh <- function(...) {
    # Computes the hyperbolic cosine of each raster cell.
    run_tool("cosh", list(...))
  }
  session$cost_allocation <- function(...) {
    # Assigns each cell to a source region using a backlink raster from cost distance analysis.
    run_tool("cost_allocation", list(...))
  }
  session$cost_distance <- function(...) {
    # Computes accumulated travel cost and backlink rasters from source and cost surfaces.
    run_tool("cost_distance", list(...))
  }
  session$cost_pathway <- function(...) {
    # Traces least-cost pathways from destination cells using a backlink raster.
    run_tool("cost_pathway", list(...))
  }
  session$count_if <- function(...) {
    # Counts the number of input rasters whose cell equals a comparison value.
    run_tool("count_if", list(...))
  }
  session$create_colour_composite <- function(...) {
    # Creates a packed RGB colour composite from red, green, blue, and optional opacity rasters.
    run_tool("create_colour_composite", list(...))
  }
  session$create_plane <- function(...) {
    # Creates a raster from a planar equation using a base raster geometry.
    run_tool("create_plane", list(...))
  }
  session$crispness_index <- function(...) {
    # Calculates the crispness index for a membership probability raster.
    run_tool("crispness_index", list(...))
  }
  session$cross_tabulation <- function(...) {
    # Performs cross-tabulation on two categorical rasters.
    run_tool("cross_tabulation", list(...))
  }
  session$csv_points_to_vector <- function(...) {
    # Imports point records from a CSV file into a point vector layer.
    run_tool("csv_points_to_vector", list(...))
  }
  session$cumulative_distribution <- function(...) {
    # Converts raster values to cumulative distribution probabilities.
    run_tool("cumulative_distribution", list(...))
  }
  session$curvedness <- function(...) {
    # Calculates the curvedness surface form descriptor from a DEM.
    run_tool("curvedness", list(...))
  }
  session$d8_flow_accum <- function(...) {
    # Calculates D8 flow accumulation from a DEM or D8 pointer raster.
    run_tool("d8_flow_accum", list(...))
  }
  session$d8_mass_flux <- function(...) {
    # Performs a D8-based mass-flux accumulation using loading, efficiency, and absorption rasters.
    run_tool("d8_mass_flux", list(...))
  }
  session$d8_pointer <- function(...) {
    # Generates a D8 flow-direction pointer raster from a DEM.
    run_tool("d8_pointer", list(...))
  }
  session$dbscan <- function(...) {
    # Performs unsupervised DBSCAN density-based clustering on a stack of input rasters.
    run_tool("dbscan", list(...))
  }
  session$decrement <- function(...) {
    # Subtracts 1 from each non-nodata raster cell.
    run_tool("decrement", list(...))
  }
  session$delete_field <- function(...) {
    # Deletes one or more attribute fields from a vector layer.
    run_tool("delete_field", list(...))
  }
  session$dem_void_filling <- function(...) {
    # Fills DEM voids using a secondary surface and interpolated elevation offsets for seamless fusion.
    run_tool("dem_void_filling", list(...))
  }
  session$densify_features <- function(...) {
    # Adds vertices along line and polygon boundaries at a specified spacing.
    run_tool("densify_features", list(...))
  }
  session$depth_in_sink <- function(...) {
    # Measures the depth each DEM cell lies below a depression-filled surface.
    run_tool("depth_in_sink", list(...))
  }
  session$depth_to_water <- function(...) {
    # Computes cartographic depth-to-water using least-cost accumulation from stream/lake source features.
    run_tool("depth_to_water", list(...))
  }
  session$deviation_from_mean_elevation <- function(...) {
    # Calculates the local topographic z-score using local mean and standard deviation.
    run_tool("deviation_from_mean_elevation", list(...))
  }
  session$deviation_from_regional_direction <- function(...) {
    # Calculates polygon directional deviation from weighted regional mean orientation and appends DEV_DIR.
    run_tool("deviation_from_regional_direction", list(...))
  }
  session$diff_of_gaussians_filter <- function(...) {
    # Performs Difference-of-Gaussians band-pass filtering.
    run_tool("diff_of_gaussians_filter", list(...))
  }
  session$difference <- function(...) {
    # Removes overlay polygon areas from input polygons using topology-based difference.
    run_tool("difference", list(...))
  }
  session$difference_curvature <- function(...) {
    # Calculates difference curvature from a DEM.
    run_tool("difference_curvature", list(...))
  }
  session$difference_from_mean_elevation <- function(...) {
    # Calculates the difference between each elevation and the local mean elevation.
    run_tool("difference_from_mean_elevation", list(...))
  }
  session$dinf_flow_accum <- function(...) {
    # Calculates D-Infinity flow accumulation from a DEM or D-Infinity pointer raster.
    run_tool("dinf_flow_accum", list(...))
  }
  session$dinf_mass_flux <- function(...) {
    # Performs a D-Infinity mass-flux accumulation using loading, efficiency, and absorption rasters.
    run_tool("dinf_mass_flux", list(...))
  }
  session$dinf_pointer <- function(...) {
    # Generates a D-Infinity flow-direction raster from a DEM.
    run_tool("dinf_pointer", list(...))
  }
  session$direct_decorrelation_stretch <- function(...) {
    # Improves packed RGB colour saturation by reducing the achromatic component and linearly stretching channels.
    run_tool("direct_decorrelation_stretch", list(...))
  }
  session$directional_relief <- function(...) {
    # Calculates directional relief by ray-tracing elevation in a specified azimuth.
    run_tool("directional_relief", list(...))
  }
  session$dissolve <- function(...) {
    # Removes shared polygon boundaries globally or by a dissolve attribute field.
    run_tool("dissolve", list(...))
  }
  session$distance_to_outlet <- function(...) {
    # Calculates downstream distance to outlet for each stream cell.
    run_tool("distance_to_outlet", list(...))
  }
  session$diversity_filter <- function(...) {
    # Computes moving-window diversity (count of unique values).
    run_tool("diversity_filter", list(...))
  }
  session$divide <- function(...) {
    # Divides the first raster by the second on a cell-by-cell basis.
    run_tool("divide", list(...))
  }
  session$downslope_distance_to_stream <- function(...) {
    # Computes downslope distance from each DEM cell to nearest stream along flow paths.
    run_tool("downslope_distance_to_stream", list(...))
  }
  session$downslope_flowpath_length <- function(...) {
    # Computes downslope flowpath length from each cell to an outlet in a D8 pointer raster.
    run_tool("downslope_flowpath_length", list(...))
  }
  session$downslope_index <- function(...) {
    # Calculates Hjerdt et al. (2004) downslope index using D8 flow directions.
    run_tool("downslope_index", list(...))
  }
  session$edge_contamination <- function(...) {
    # Identifies DEM cells whose upslope area extends beyond the DEM edge for common flow-routing schemes.
    run_tool("edge_contamination", list(...))
  }
  session$edge_density <- function(...) {
    # Calculates local density of breaks-in-slope using angular normal-vector differences.
    run_tool("edge_density", list(...))
  }
  session$edge_preserving_mean_filter <- function(...) {
    # Performs thresholded edge-preserving mean filtering.
    run_tool("edge_preserving_mean_filter", list(...))
  }
  session$edge_proportion <- function(...) {
    # Calculates the proportion of each patch's cells that are edge cells and maps it back to patch cells.
    run_tool("edge_proportion", list(...))
  }
  session$elev_above_pit <- function(...) {
    # Calculates elevation above the nearest downslope pit cell (or edge sink).
    run_tool("elev_above_pit", list(...))
  }
  session$elev_above_pit_dist <- function(...) {
    # Compatibility alias for elev_above_pit.
    run_tool("elev_above_pit_dist", list(...))
  }
  session$elev_relative_to_min_max <- function(...) {
    # Expresses each elevation as a percentage (0–100) of the raster's elevation range.
    run_tool("elev_relative_to_min_max", list(...))
  }
  session$elev_relative_to_watershed_min_max <- function(...) {
    # Calculates a DEM cell's relative elevation position within each watershed as a percentage.
    run_tool("elev_relative_to_watershed_min_max", list(...))
  }
  session$elevation_above_stream <- function(...) {
    # Computes elevation above nearest stream measured along downslope flow paths.
    run_tool("elevation_above_stream", list(...))
  }
  session$elevation_above_stream_euclidean <- function(...) {
    # Computes elevation above nearest stream using straight-line (Euclidean) proximity.
    run_tool("elevation_above_stream_euclidean", list(...))
  }
  session$elevation_percentile <- function(...) {
    # Calculates the local percentile rank of each cell elevation within a neighbourhood window.
    run_tool("elevation_percentile", list(...))
  }
  session$eliminate_coincident_points <- function(...) {
    # Removes coincident or near-coincident points within a tolerance distance.
    run_tool("eliminate_coincident_points", list(...))
  }
  session$elongation_ratio <- function(...) {
    # Computes elongation ratio (short axis / long axis of bounding rectangle) for polygon features.
    run_tool("elongation_ratio", list(...))
  }
  session$embankment_mapping <- function(...) {
    # Maps transportation embankments from a DEM and road network, with optional embankment-surface removal via interpolation. Authored by John Lindsay and Nigel VanNieuwenhuizen.
    run_tool("embankment_mapping", list(...))
  }
  session$emboss_filter <- function(...) {
    # Performs directional emboss filtering.
    run_tool("emboss_filter", list(...))
  }
  session$equal_to <- function(...) {
    # Tests whether two rasters are equal on a cell-by-cell basis.
    run_tool("equal_to", list(...))
  }
  session$erase <- function(...) {
    # Erases overlay polygon areas from input polygons and preserves input attributes.
    run_tool("erase", list(...))
  }
  session$erase_polygon_from_lidar <- function(...) {
    # Removes LiDAR points that fall within polygon geometry.
    run_tool("erase_polygon_from_lidar", list(...))
  }
  session$erase_polygon_from_raster <- function(...) {
    # Sets raster cells inside polygons to NoData while preserving cells in polygon holes.
    run_tool("erase_polygon_from_raster", list(...))
  }
  session$euclidean_allocation <- function(...) {
    # Assigns each valid cell the value of its nearest non-zero target cell.
    run_tool("euclidean_allocation", list(...))
  }
  session$euclidean_distance <- function(...) {
    # Computes Euclidean distance to nearest non-zero target cell in a raster.
    run_tool("euclidean_distance", list(...))
  }
  session$evaluate_training_sites <- function(...) {
    # Evaluates class separability in multi-band training polygons and writes an HTML report with per-band distribution statistics.
    run_tool("evaluate_training_sites", list(...))
  }
  session$exp <- function(...) {
    # Computes e raised to the power of each raster cell.
    run_tool("exp", list(...))
  }
  session$exp2 <- function(...) {
    # Computes 2 raised to the power of each raster cell.
    run_tool("exp2", list(...))
  }
  session$export_table_to_csv <- function(...) {
    # Exports a vector attribute table to a CSV file.
    run_tool("export_table_to_csv", list(...))
  }
  session$exposure_towards_wind_flux <- function(...) {
    # Calculates terrain exposure relative to dominant wind direction and upwind horizon shielding.
    run_tool("exposure_towards_wind_flux", list(...))
  }
  session$extend_vector_lines <- function(...) {
    # Extends polyline endpoints by a specified distance at the start, end, or both.
    run_tool("extend_vector_lines", list(...))
  }
  session$extract_by_attribute <- function(...) {
    # Extracts vector features that satisfy an attribute expression.
    run_tool("extract_by_attribute", list(...))
  }
  session$extract_nodes <- function(...) {
    # Converts polyline and polygon vertices into point features.
    run_tool("extract_nodes", list(...))
  }
  session$extract_raster_values_at_points <- function(...) {
    # Samples one or more rasters at point locations and writes the values to point attributes.
    run_tool("extract_raster_values_at_points", list(...))
  }
  session$extract_streams <- function(...) {
    # Extracts streams based on flow accumulation threshold.
    run_tool("extract_streams", list(...))
  }
  session$extract_valleys <- function(...) {
    # Extracts valleys from DEM.
    run_tool("extract_valleys", list(...))
  }
  session$farthest_channel_head <- function(...) {
    # Calculates distance to most distant channel head.
    run_tool("farthest_channel_head", list(...))
  }
  session$fast_almost_gaussian_filter <- function(...) {
    # Performs a fast approximation to Gaussian smoothing.
    run_tool("fast_almost_gaussian_filter", list(...))
  }
  session$fd8_flow_accum <- function(...) {
    # Calculates FD8 flow accumulation from a DEM.
    run_tool("fd8_flow_accum", list(...))
  }
  session$fd8_pointer <- function(...) {
    # Generates an FD8 multiple-flow-direction pointer raster from a DEM.
    run_tool("fd8_pointer", list(...))
  }
  session$feature_preserving_smoothing <- function(...) {
    # Smooths DEM roughness while preserving breaks-in-slope using normal-vector filtering.
    run_tool("feature_preserving_smoothing", list(...))
  }
  session$fetch_analysis <- function(...) {
    # Computes upwind distance to the first topographic obstacle along a specified azimuth.
    run_tool("fetch_analysis", list(...))
  }
  session$field_calculator <- function(...) {
    # Calculates a field value from an expression using feature attributes and geometry variables.
    run_tool("field_calculator", list(...))
  }
  session$fill_burn <- function(...) {
    # Hydro-enforces a DEM by burning streams and then filling depressions.
    run_tool("fill_burn", list(...))
  }
  session$fill_depressions <- function(...) {
    # Fills depressions in a DEM using a priority-flood strategy with optional flat resolution.
    run_tool("fill_depressions", list(...))
  }
  session$fill_depressions_planchon_and_darboux <- function(...) {
    # Fills depressions in a DEM with a Planchon-and-Darboux-compatible interface.
    run_tool("fill_depressions_planchon_and_darboux", list(...))
  }
  session$fill_depressions_wang_and_liu <- function(...) {
    # Fills depressions in a DEM with a Wang-and-Liu-compatible interface.
    run_tool("fill_depressions_wang_and_liu", list(...))
  }
  session$fill_missing_data <- function(...) {
    # Fills NoData gaps using inverse-distance weighting from valid gap-edge cells.
    run_tool("fill_missing_data", list(...))
  }
  session$fill_pits <- function(...) {
    # Fills single-cell pits in a DEM.
    run_tool("fill_pits", list(...))
  }
  session$filter_lidar <- function(...) {
    # Filters LiDAR points using a boolean expression over point attributes.
    run_tool("filter_lidar", list(...))
  }
  session$filter_lidar_by_percentile <- function(...) {
    # Selects one representative point per grid block based on elevation percentile.
    run_tool("filter_lidar_by_percentile", list(...))
  }
  session$filter_lidar_by_reference_surface <- function(...) {
    # Extracts or classifies points based on z relation to a reference raster surface.
    run_tool("filter_lidar_by_reference_surface", list(...))
  }
  session$filter_lidar_classes <- function(...) {
    # Removes points that match excluded classification values.
    run_tool("filter_lidar_classes", list(...))
  }
  session$filter_lidar_noise <- function(...) {
    # Removes low (class 7) and high (class 18) noise-classified points from a LiDAR file.
    run_tool("filter_lidar_noise", list(...))
  }
  session$filter_lidar_scan_angles <- function(...) {
    # Removes LiDAR points whose absolute scan angle exceeds a threshold.
    run_tool("filter_lidar_scan_angles", list(...))
  }
  session$filter_raster_features_by_area <- function(...) {
    # Removes integer-labelled raster features smaller than a cell-count threshold.
    run_tool("filter_raster_features_by_area", list(...))
  }
  session$filter_vector_features_by_area <- function(...) {
    # Filters polygon features below a minimum area threshold.
    run_tool("filter_vector_features_by_area", list(...))
  }
  session$find_flightline_edge_points <- function(...) {
    # Extracts only points flagged as edge-of-flightline.
    run_tool("find_flightline_edge_points", list(...))
  }
  session$find_lowest_or_highest_points <- function(...) {
    # Locates lowest and/or highest raster cells and outputs their locations as points.
    run_tool("find_lowest_or_highest_points", list(...))
  }
  session$find_main_stem <- function(...) {
    # Identifies main stem of stream network.
    run_tool("find_main_stem", list(...))
  }
  session$find_noflow_cells <- function(...) {
    # Finds DEM cells that have no lower D8 neighbour.
    run_tool("find_noflow_cells", list(...))
  }
  session$find_parallel_flow <- function(...) {
    # Identifies stream cells that possess parallel D8 flow directions.
    run_tool("find_parallel_flow", list(...))
  }
  session$find_patch_edge_cells <- function(...) {
    # Identifies edge cells for each positive raster patch ID; non-edge patch cells are set to zero.
    run_tool("find_patch_edge_cells", list(...))
  }
  session$find_ridges <- function(...) {
    # Identifies potential ridge and peak cells in a DEM, with optional line thinning.
    run_tool("find_ridges", list(...))
  }
  session$fix_dangling_arcs <- function(...) {
    # Fixes undershot and overshot dangling arcs in a line network by snapping line endpoints within a threshold distance.
    run_tool("fix_dangling_arcs", list(...))
  }
  session$flatten_lakes <- function(...) {
    # Flattens lake elevations using minimum perimeter elevation for each polygon.
    run_tool("flatten_lakes", list(...))
  }
  session$flightline_overlap <- function(...) {
    # Counts distinct point-source IDs per raster cell to identify overlapping flightlines.
    run_tool("flightline_overlap", list(...))
  }
  session$flip_image <- function(...) {
    # Flips an image vertically, horizontally, or both.
    run_tool("flip_image", list(...))
  }
  session$flood_order <- function(...) {
    # Outputs the sequential priority-flood order for each DEM cell.
    run_tool("flood_order", list(...))
  }
  session$floor <- function(...) {
    # Rounds each raster cell downward to the nearest integer.
    run_tool("floor", list(...))
  }
  session$flow_accum_full_workflow <- function(...) {
    # Runs a full non-divergent flow-accumulation workflow and returns breached DEM, flow-direction pointer, and accumulation.
    run_tool("flow_accum_full_workflow", list(...))
  }
  session$flow_length_diff <- function(...) {
    # Computes local maximum absolute differences in downslope path length from a D8 pointer raster.
    run_tool("flow_length_diff", list(...))
  }
  session$frangi_filter <- function(...) {
    # Performs multiscale Frangi vesselness enhancement.
    run_tool("frangi_filter", list(...))
  }
  session$frost_filter <- function(...) {
    # Performs adaptive Frost speckle filtering for radar imagery.
    run_tool("frost_filter", list(...))
  }
  session$fuzzy_knn_classification <- function(...) {
    # Performs fuzzy k-nearest-neighbor classification and outputs class membership confidence.
    run_tool("fuzzy_knn_classification", list(...))
  }
  session$gabor_filter_bank <- function(...) {
    # Performs multi-orientation Gabor response filtering.
    run_tool("gabor_filter_bank", list(...))
  }
  session$gamma_correction <- function(...) {
    # Applies gamma intensity correction to grayscale or RGB imagery.
    run_tool("gamma_correction", list(...))
  }
  session$gamma_map_filter <- function(...) {
    # Performs Gamma-MAP speckle filtering for radar imagery.
    run_tool("gamma_map_filter", list(...))
  }
  session$gaussian_contrast_stretch <- function(...) {
    # Stretches contrast by matching to a Gaussian reference distribution.
    run_tool("gaussian_contrast_stretch", list(...))
  }
  session$gaussian_curvature <- function(...) {
    # Calculates Gaussian curvature from a DEM.
    run_tool("gaussian_curvature", list(...))
  }
  session$gaussian_filter <- function(...) {
    # Performs Gaussian smoothing on a raster image.
    run_tool("gaussian_filter", list(...))
  }
  session$generalize_classified_raster <- function(...) {
    # Generalizes small class patches by merging them into neighboring larger classes.
    run_tool("generalize_classified_raster", list(...))
  }
  session$generalize_with_similarity <- function(...) {
    # Generalizes small patches in a classified raster by merging them into the most spectrally similar neighboring patch.
    run_tool("generalize_with_similarity", list(...))
  }
  session$generating_function <- function(...) {
    # Calculates generating function from a DEM.
    run_tool("generating_function", list(...))
  }
  session$geomorphons <- function(...) {
    # Classifies landforms using 8-direction line-of-sight ternary patterns based on zenith-nadir angle differences, or 10 common geomorphon forms. Authored by Dan Newman and John Lindsay.
    run_tool("geomorphons", list(...))
  }
  session$greater_than <- function(...) {
    # Tests whether the first raster is greater than the second on a cell-by-cell basis.
    run_tool("greater_than", list(...))
  }
  session$guided_filter <- function(...) {
    # Performs edge-preserving guided filtering using local linear models.
    run_tool("guided_filter", list(...))
  }
  session$hack_stream_order <- function(...) {
    # Assigns Hack stream order to stream cells.
    run_tool("hack_stream_order", list(...))
  }
  session$heat_map <- function(...) {
    # Generates a kernel-density heat map raster from point occurrences.
    run_tool("heat_map", list(...))
  }
  session$height_above_ground <- function(...) {
    # Converts LiDAR elevations to heights above the nearest ground-classified point.
    run_tool("height_above_ground", list(...))
  }
  session$hexagonal_grid_from_raster_base <- function(...) {
    # Creates a hexagonal polygon grid covering a raster extent.
    run_tool("hexagonal_grid_from_raster_base", list(...))
  }
  session$hexagonal_grid_from_vector_base <- function(...) {
    # Creates a hexagonal polygon grid covering a vector-layer bounding extent.
    run_tool("hexagonal_grid_from_vector_base", list(...))
  }
  session$high_pass_bilateral_filter <- function(...) {
    # Computes a high-pass residual by subtracting bilateral smoothing from the input raster.
    run_tool("high_pass_bilateral_filter", list(...))
  }
  session$high_pass_filter <- function(...) {
    # Performs high-pass filtering using neighborhood mean subtraction.
    run_tool("high_pass_filter", list(...))
  }
  session$high_pass_median_filter <- function(...) {
    # Performs high-pass filtering by subtracting local median from center values.
    run_tool("high_pass_median_filter", list(...))
  }
  session$highest_position <- function(...) {
    # Returns the zero-based raster-stack index containing the highest value at each cell.
    run_tool("highest_position", list(...))
  }
  session$hillshade <- function(...) {
    # Produces shaded-relief from a DEM.
    run_tool("hillshade", list(...))
  }
  session$hillslopes <- function(...) {
    # Identifies hillslope regions draining to each stream link, separating left- and right-bank areas.
    run_tool("hillslopes", list(...))
  }
  session$histogram_equalization <- function(...) {
    # Applies histogram equalization to improve image contrast.
    run_tool("histogram_equalization", list(...))
  }
  session$histogram_matching <- function(...) {
    # Matches an image histogram to a supplied reference histogram.
    run_tool("histogram_matching", list(...))
  }
  session$histogram_matching_two_images <- function(...) {
    # Matches an input image histogram to a reference image histogram.
    run_tool("histogram_matching_two_images", list(...))
  }
  session$hole_proportion <- function(...) {
    # Calculates polygon hole area divided by hull area and appends HOLE_PROP.
    run_tool("hole_proportion", list(...))
  }
  session$horizon_angle <- function(...) {
    # Calculates horizon angle (maximum slope) along a specified azimuth direction.
    run_tool("horizon_angle", list(...))
  }
  session$horizontal_excess_curvature <- function(...) {
    # Calculates horizontal excess curvature from a DEM.
    run_tool("horizontal_excess_curvature", list(...))
  }
  session$horton_ratios <- function(...) {
    # Calculates Horton bifurcation, length, drainage-area, and slope ratios.
    run_tool("horton_ratios", list(...))
  }
  session$horton_stream_order <- function(...) {
    # Assigns Horton stream order to stream cells.
    run_tool("horton_stream_order", list(...))
  }
  session$hydrologic_connectivity <- function(...) {
    # Computes DUL and UDSA connectivity indices from a DEM.
    run_tool("hydrologic_connectivity", list(...))
  }
  session$hypsometric_analysis <- function(...) {
    # Creates a hypsometric (area-elevation) curve HTML report for one or more DEMs.
    run_tool("hypsometric_analysis", list(...))
  }
  session$hypsometrically_tinted_hillshade <- function(...) {
    # Creates a Swiss-style terrain rendering by blending multi-azimuth hillshade with hypsometric tinting and optional atmospheric haze.
    run_tool("hypsometrically_tinted_hillshade", list(...))
  }
  session$idw_interpolation <- function(...) {
    # Interpolates a raster from point samples using inverse-distance weighting.
    run_tool("idw_interpolation", list(...))
  }
  session$ihs_to_rgb <- function(...) {
    # Converts intensity, hue, and saturation band rasters back to red, green, and blue channels (0–255).
    run_tool("ihs_to_rgb", list(...))
  }
  session$image_autocorrelation <- function(...) {
    # Computes Moran's I for one or more raster images.
    run_tool("image_autocorrelation", list(...))
  }
  session$image_correlation <- function(...) {
    # Computes Pearson correlation matrix for two or more raster images.
    run_tool("image_correlation", list(...))
  }
  session$image_correlation_neighbourhood_analysis <- function(...) {
    # Performs moving-window correlation analysis between two rasters and returns correlation and p-value rasters.
    run_tool("image_correlation_neighbourhood_analysis", list(...))
  }
  session$image_regression <- function(...) {
    # Performs bivariate linear regression between two rasters and outputs a residual raster and report.
    run_tool("image_regression", list(...))
  }
  session$image_segmentation <- function(...) {
    # Segments multi-band raster stacks into contiguous homogeneous regions using seeded region growing.
    run_tool("image_segmentation", list(...))
  }
  session$image_slider <- function(...) {
    # Creates an interactive HTML image slider from two raster images.
    run_tool("image_slider", list(...))
  }
  session$image_stack_profile <- function(...) {
    # Extracts per-point profiles across an ordered raster stack and optionally writes an HTML report.
    run_tool("image_stack_profile", list(...))
  }
  session$impoundment_size_index <- function(...) {
    # Computes mean/max depth, volume, area, and dam-height impoundment metrics.
    run_tool("impoundment_size_index", list(...))
  }
  session$improved_ground_point_filter <- function(...) {
    # Multi-stage ground point filtering pipeline.
    run_tool("improved_ground_point_filter", list(...))
  }
  session$increment <- function(...) {
    # Adds 1 to each non-nodata raster cell.
    run_tool("increment", list(...))
  }
  session$individual_tree_detection <- function(...) {
    # Identifies tree top points in a LiDAR cloud using local maxima detection.
    run_tool("individual_tree_detection", list(...))
  }
  session$individual_tree_segmentation <- function(...) {
    # Segments vegetation LiDAR points into individual tree clusters using a mean-shift mode-seeking workflow.
    run_tool("individual_tree_segmentation", list(...))
  }
  session$inplace_add <- function(...) {
    # Performs an in-place addition operation (input1 += input2).
    run_tool("inplace_add", list(...))
  }
  session$inplace_divide <- function(...) {
    # Performs an in-place division operation (input1 /= input2).
    run_tool("inplace_divide", list(...))
  }
  session$inplace_multiply <- function(...) {
    # Performs an in-place multiplication operation (input1 *= input2).
    run_tool("inplace_multiply", list(...))
  }
  session$inplace_subtract <- function(...) {
    # Performs an in-place subtraction operation (input1 -= input2).
    run_tool("inplace_subtract", list(...))
  }
  session$insert_dams <- function(...) {
    # Adds local dam embankments at specified points using profile-based crest selection.
    run_tool("insert_dams", list(...))
  }
  session$integer_division <- function(...) {
    # Divides two rasters and truncates each result toward zero.
    run_tool("integer_division", list(...))
  }
  session$integral_image_transform <- function(...) {
    # Computes a summed-area (integral image) transform for each band.
    run_tool("integral_image_transform", list(...))
  }
  session$intersect <- function(...) {
    # Intersects input and overlay polygons using topology-based overlay and tracks source feature IDs.
    run_tool("intersect", list(...))
  }
  session$inverse_pca <- function(...) {
    # Reconstructs original band images from PCA component rasters using stored eigenvectors.
    run_tool("inverse_pca", list(...))
  }
  session$is_nodata <- function(...) {
    # Outputs 1 for nodata cells and 0 for all valid cells.
    run_tool("is_nodata", list(...))
  }
  session$isobasins <- function(...) {
    # Divides a landscape into approximately equal-sized watersheds (isobasins) based on a target area threshold.
    run_tool("isobasins", list(...))
  }
  session$jenson_snap_pour_points <- function(...) {
    # Snaps each pour point to the nearest stream cell within a search distance, preserving all input attributes.
    run_tool("jenson_snap_pour_points", list(...))
  }
  session$join_tables <- function(...) {
    # Joins attributes from a foreign vector table to a primary vector table using key fields.
    run_tool("join_tables", list(...))
  }
  session$k_means_clustering <- function(...) {
    # Performs k-means clustering on a multi-band raster stack and outputs a categorical class raster.
    run_tool("k_means_clustering", list(...))
  }
  session$k_nearest_mean_filter <- function(...) {
    # Performs edge-preserving k-nearest neighbor mean smoothing.
    run_tool("k_nearest_mean_filter", list(...))
  }
  session$k_shortest_paths_network <- function(...) {
    # Finds the k shortest simple paths between start and end coordinates over a line network.
    run_tool("k_shortest_paths_network", list(...))
  }
  session$kappa_index <- function(...) {
    # Computes Cohen's kappa and agreement metrics between two categorical rasters.
    run_tool("kappa_index", list(...))
  }
  session$knn_classification <- function(...) {
    # Performs supervised k-nearest-neighbor classification on multi-band input rasters.
    run_tool("knn_classification", list(...))
  }
  session$knn_regression <- function(...) {
    # Performs supervised k-nearest-neighbor regression on multi-band input rasters.
    run_tool("knn_regression", list(...))
  }
  session$ks_normality_test <- function(...) {
    # Evaluates whether raster values are drawn from a normal distribution.
    run_tool("ks_normality_test", list(...))
  }
  session$kuan_filter <- function(...) {
    # Performs Kuan speckle filtering for radar imagery.
    run_tool("kuan_filter", list(...))
  }
  session$kuwahara_filter <- function(...) {
    # Performs edge-preserving Kuwahara filtering using minimum-variance subwindows.
    run_tool("kuwahara_filter", list(...))
  }
  session$laplacian_filter <- function(...) {
    # Performs Laplacian edge/sharpen filtering.
    run_tool("laplacian_filter", list(...))
  }
  session$laplacian_of_gaussians_filter <- function(...) {
    # Performs Laplacian-of-Gaussians edge enhancement.
    run_tool("laplacian_of_gaussians_filter", list(...))
  }
  session$las_to_ascii <- function(...) {
    # Converts LiDAR points to CSV ASCII text.
    run_tool("las_to_ascii", list(...))
  }
  session$las_to_shapefile <- function(...) {
    # Converts LAS/LAZ point clouds into vector point shapefiles.
    run_tool("las_to_shapefile", list(...))
  }
  session$layer_footprint_raster <- function(...) {
    # Creates a polygon footprint representing the full extent of an input raster.
    run_tool("layer_footprint_raster", list(...))
  }
  session$layer_footprint_vector <- function(...) {
    # Creates a polygon footprint representing the full bounding extent of an input vector layer.
    run_tool("layer_footprint_vector", list(...))
  }
  session$lee_filter <- function(...) {
    # Performs Lee sigma filtering using in-range neighborhood averaging.
    run_tool("lee_filter", list(...))
  }
  session$length_of_upstream_channels <- function(...) {
    # Calculates total upstream channel length.
    run_tool("length_of_upstream_channels", list(...))
  }
  session$less_than <- function(...) {
    # Tests whether the first raster is less than the second on a cell-by-cell basis.
    run_tool("less_than", list(...))
  }
  session$lidar_block_maximum <- function(...) {
    # Creates a raster by assigning each cell the maximum value of included LiDAR points.
    run_tool("lidar_block_maximum", list(...))
  }
  session$lidar_block_minimum <- function(...) {
    # Creates a raster by assigning each cell the minimum value of included LiDAR points.
    run_tool("lidar_block_minimum", list(...))
  }
  session$lidar_classify_subset <- function(...) {
    # Classifies points in a base LiDAR cloud that spatially match points in a subset cloud.
    run_tool("lidar_classify_subset", list(...))
  }
  session$lidar_colourize <- function(...) {
    # Assigns LiDAR point RGB values from an overlapping raster image.
    run_tool("lidar_colourize", list(...))
  }
  session$lidar_construct_vector_tin <- function(...) {
    # Creates a vector TIN (triangular mesh) from LiDAR points.
    run_tool("lidar_construct_vector_tin", list(...))
  }
  session$lidar_contour <- function(...) {
    # Creates contour vector lines from a LiDAR point cloud using TIN contouring.
    run_tool("lidar_contour", list(...))
  }
  session$lidar_digital_surface_model <- function(...) {
    # Builds a DSM from top-surface LiDAR points and TIN interpolation.
    run_tool("lidar_digital_surface_model", list(...))
  }
  session$lidar_eigenvalue_features <- function(...) {
    # Computes local PCA-based LiDAR neighbourhood features and writes a .eigen binary with JSON sidecar.
    run_tool("lidar_eigenvalue_features", list(...))
  }
  session$lidar_elevation_slice <- function(...) {
    # Extracts or reclassifies LiDAR points within a specified elevation range.
    run_tool("lidar_elevation_slice", list(...))
  }
  session$lidar_ground_point_filter <- function(...) {
    # Slope-based filtering/classification of off-terrain points in LiDAR data.
    run_tool("lidar_ground_point_filter", list(...))
  }
  session$lidar_hex_bin <- function(...) {
    # Bins LiDAR points into a hexagonal grid and outputs per-cell summary attributes.
    run_tool("lidar_hex_bin", list(...))
  }
  session$lidar_hillshade <- function(...) {
    # Creates a hillshade raster from LiDAR elevations using local block maxima as surface input.
    run_tool("lidar_hillshade", list(...))
  }
  session$lidar_histogram <- function(...) {
    # Builds a simple histogram report for a selected LiDAR attribute.
    run_tool("lidar_histogram", list(...))
  }
  session$lidar_idw_interpolation <- function(...) {
    # Interpolates a raster from LiDAR points using inverse-distance weighting.
    run_tool("lidar_idw_interpolation", list(...))
  }
  session$lidar_info <- function(...) {
    # Generates a textual or HTML summary report for a LiDAR file.
    run_tool("lidar_info", list(...))
  }
  session$lidar_join <- function(...) {
    # Merges multiple LiDAR files into a single output point cloud.
    run_tool("lidar_join", list(...))
  }
  session$lidar_kappa <- function(...) {
    # Computes a kappa agreement report between two classified LiDAR clouds and writes a class-agreement raster.
    run_tool("lidar_kappa", list(...))
  }
  session$lidar_nearest_neighbour_gridding <- function(...) {
    # Interpolates a raster from LiDAR points using nearest-neighbour assignment.
    run_tool("lidar_nearest_neighbour_gridding", list(...))
  }
  session$lidar_point_density <- function(...) {
    # Computes point density from LiDAR samples within a moving-radius neighbourhood.
    run_tool("lidar_point_density", list(...))
  }
  session$lidar_point_return_analysis <- function(...) {
    # Runs return-sequence QC analysis and writes a text report; optionally writes a classified QC LiDAR output.
    run_tool("lidar_point_return_analysis", list(...))
  }
  session$lidar_point_stats <- function(...) {
    # Creates one or more raster grids summarizing LiDAR point distributions.
    run_tool("lidar_point_stats", list(...))
  }
  session$lidar_radial_basis_function_interpolation <- function(...) {
    # Interpolates a raster from LiDAR points using local radial-basis similarity weighting.
    run_tool("lidar_radial_basis_function_interpolation", list(...))
  }
  session$lidar_ransac_planes <- function(...) {
    # Identifies locally planar LiDAR points using neighbourhood RANSAC plane fitting.
    run_tool("lidar_ransac_planes", list(...))
  }
  session$lidar_remove_outliers <- function(...) {
    # Filters or classifies outlier points based on local elevation residuals.
    run_tool("lidar_remove_outliers", list(...))
  }
  session$lidar_rooftop_analysis <- function(...) {
    # Identifies planar rooftop segments within building footprints and outputs segment polygons with roof attributes.
    run_tool("lidar_rooftop_analysis", list(...))
  }
  session$lidar_segmentation <- function(...) {
    # Segments a LiDAR cloud into connected components and assigns segment colours.
    run_tool("lidar_segmentation", list(...))
  }
  session$lidar_segmentation_based_filter <- function(...) {
    # Ground-point filtering based on neighbourhood-connected low-relief segments.
    run_tool("lidar_segmentation_based_filter", list(...))
  }
  session$lidar_shift <- function(...) {
    # Shifts LiDAR point coordinates by x/y/z offsets.
    run_tool("lidar_shift", list(...))
  }
  session$lidar_sibson_interpolation <- function(...) {
    # Interpolates a raster from LiDAR points using true Sibson natural-neighbour interpolation.
    run_tool("lidar_sibson_interpolation", list(...))
  }
  session$lidar_thin <- function(...) {
    # Thins a LiDAR point cloud by retaining at most one point per grid cell.
    run_tool("lidar_thin", list(...))
  }
  session$lidar_thin_high_density <- function(...) {
    # Thins points in locally high-density areas while preserving lower-density regions.
    run_tool("lidar_thin_high_density", list(...))
  }
  session$lidar_tile <- function(...) {
    # Splits an input LiDAR file into a regular tile grid and writes one output per populated tile.
    run_tool("lidar_tile", list(...))
  }
  session$lidar_tile_footprint <- function(...) {
    # Creates polygon footprints (bounding boxes or convex hulls) for LiDAR tiles.
    run_tool("lidar_tile_footprint", list(...))
  }
  session$lidar_tin_gridding <- function(...) {
    # Interpolates a raster from LiDAR points using Delaunay triangulation.
    run_tool("lidar_tin_gridding", list(...))
  }
  session$lidar_tophat_transform <- function(...) {
    # Applies a white top-hat transform to LiDAR elevations to approximate height above local ground.
    run_tool("lidar_tophat_transform", list(...))
  }
  session$line_detection_filter <- function(...) {
    # Performs directional line detection.
    run_tool("line_detection_filter", list(...))
  }
  session$line_intersections <- function(...) {
    # Finds line intersection points between input and overlay layers and appends parent IDs with merged attributes.
    run_tool("line_intersections", list(...))
  }
  session$line_polygon_clip <- function(...) {
    # Clips line features to polygon interiors and outputs clipped line segments.
    run_tool("line_polygon_clip", list(...))
  }
  session$line_thinning <- function(...) {
    # Reduces connected binary raster features to one-cell-wide skeleton lines.
    run_tool("line_thinning", list(...))
  }
  session$linearity_index <- function(...) {
    # Computes linearity index (straight-line distance / actual length) for line and polygon features.
    run_tool("linearity_index", list(...))
  }
  session$lines_to_polygons <- function(...) {
    # Converts polyline features into polygon features, treating the first part as the exterior ring and later parts as holes.
    run_tool("lines_to_polygons", list(...))
  }
  session$list_unique_values <- function(...) {
    # Lists unique values and frequencies in a vector attribute field.
    run_tool("list_unique_values", list(...))
  }
  session$list_unique_values_raster <- function(...) {
    # Lists unique valid values in a raster (capped to protect memory).
    run_tool("list_unique_values_raster", list(...))
  }
  session$ln <- function(...) {
    # Computes the natural logarithm of each raster cell.
    run_tool("ln", list(...))
  }
  session$locate_points_along_routes <- function(...) {
    # Locates point features along route lines and writes route-measure attributes.
    run_tool("locate_points_along_routes", list(...))
  }
  session$log10 <- function(...) {
    # Computes the base-10 logarithm of each raster cell.
    run_tool("log10", list(...))
  }
  session$log2 <- function(...) {
    # Computes the base-2 logarithm of each raster cell.
    run_tool("log2", list(...))
  }
  session$logistic_regression <- function(...) {
    # Performs supervised logistic regression classification on multi-band input rasters.
    run_tool("logistic_regression", list(...))
  }
  session$long_profile <- function(...) {
    # Creates longitudinal stream profile.
    run_tool("long_profile", list(...))
  }
  session$long_profile_from_points <- function(...) {
    # Creates long profile from vector points.
    run_tool("long_profile_from_points", list(...))
  }
  session$longest_flowpath <- function(...) {
    # Delineates longest flowpath lines for each basin in a basin raster.
    run_tool("longest_flowpath", list(...))
  }
  session$lowest_position <- function(...) {
    # Returns the zero-based raster-stack index containing the lowest value at each cell.
    run_tool("lowest_position", list(...))
  }
  session$majority_filter <- function(...) {
    # Computes moving-window mode (majority class/value).
    run_tool("majority_filter", list(...))
  }
  session$map_features <- function(...) {
    # Maps discrete elevated terrain features from a raster using descending-priority region growth.
    run_tool("map_features", list(...))
  }
  session$map_matching_v1 <- function(...) {
    # Snaps trajectory points onto a line network and reconstructs an inferred route with diagnostics.
    run_tool("map_matching_v1", list(...))
  }
  session$map_off_terrain_objects <- function(...) {
    # Maps off-terrain object segments in DSMs using slope-constrained region growing and optional minimum feature-size filtering.
    run_tool("map_off_terrain_objects", list(...))
  }
  session$max <- function(...) {
    # Performs a MAX operation on two rasters or a raster and a constant value.
    run_tool("max", list(...))
  }
  session$max_absolute_overlay <- function(...) {
    # Computes the per-cell maximum absolute value across a raster stack, propagating NoData if any input cell is NoData.
    run_tool("max_absolute_overlay", list(...))
  }
  session$max_anisotropy_dev <- function(...) {
    # Calculates maximum anisotropy in elevation deviation over a range of neighbourhood scales. Written by Dan Newman.
    run_tool("max_anisotropy_dev", list(...))
  }
  session$max_anisotropy_dev_signature <- function(...) {
    # Calculates multiscale anisotropy signatures for input point sites and writes an HTML report. Written by Dan Newman.
    run_tool("max_anisotropy_dev_signature", list(...))
  }
  session$max_branch_length <- function(...) {
    # Calculates maximum branch length between neighbouring D8 flowpaths, useful for highlighting divides.
    run_tool("max_branch_length", list(...))
  }
  session$max_difference_from_mean <- function(...) {
    # Calculates maximum absolute difference-from-mean over a range of neighbourhood scales.
    run_tool("max_difference_from_mean", list(...))
  }
  session$max_downslope_elev_change <- function(...) {
    # Calculates the maximum elevation drop to lower neighbouring cells.
    run_tool("max_downslope_elev_change", list(...))
  }
  session$max_elev_dev_signature <- function(...) {
    # Calculates multiscale elevation-deviation signatures for input point sites and writes an HTML report.
    run_tool("max_elev_dev_signature", list(...))
  }
  session$max_elevation_deviation <- function(...) {
    # Calculates maximum standardized elevation deviation (DEVmax) over a range of neighbourhood scales.
    run_tool("max_elevation_deviation", list(...))
  }
  session$max_overlay <- function(...) {
    # Computes the per-cell maximum across a raster stack, propagating NoData if any input cell is NoData.
    run_tool("max_overlay", list(...))
  }
  session$max_upslope_elev_change <- function(...) {
    # Calculates the maximum elevation gain to higher neighbouring cells.
    run_tool("max_upslope_elev_change", list(...))
  }
  session$max_upslope_flowpath_length <- function(...) {
    # Computes the maximum upslope flowpath length passing through each DEM cell.
    run_tool("max_upslope_flowpath_length", list(...))
  }
  session$max_upslope_value <- function(...) {
    # Propagates maximum upslope value along D8 flowpaths over a DEM.
    run_tool("max_upslope_value", list(...))
  }
  session$maximal_curvature <- function(...) {
    # Calculates maximal (maximum principal) curvature from a DEM.
    run_tool("maximal_curvature", list(...))
  }
  session$maximum_filter <- function(...) {
    # Computes a moving-window maximum for each raster cell.
    run_tool("maximum_filter", list(...))
  }
  session$mdinf_flow_accum <- function(...) {
    # Calculates MD-Infinity triangular multiple-flow-direction accumulation from a DEM.
    run_tool("mdinf_flow_accum", list(...))
  }
  session$mean_curvature <- function(...) {
    # Calculates mean curvature from a DEM.
    run_tool("mean_curvature", list(...))
  }
  session$mean_filter <- function(...) {
    # Computes a moving-window mean for each raster cell.
    run_tool("mean_filter", list(...))
  }
  session$median_filter <- function(...) {
    # Computes moving-window median values.
    run_tool("median_filter", list(...))
  }
  session$medoid <- function(...) {
    # Calculates medoid points from vector geometries.
    run_tool("medoid", list(...))
  }
  session$merge_line_segments <- function(...) {
    # Merges connected line segments that meet at non-branching endpoints.
    run_tool("merge_line_segments", list(...))
  }
  session$merge_table_with_csv <- function(...) {
    # Merges attributes from a CSV table into a vector attribute table by key fields.
    run_tool("merge_table_with_csv", list(...))
  }
  session$merge_vectors <- function(...) {
    # Combines two or more input vectors of the same geometry type into a single output vector.
    run_tool("merge_vectors", list(...))
  }
  session$min <- function(...) {
    # Performs a MIN operation on two rasters or a raster and a constant value.
    run_tool("min", list(...))
  }
  session$min_absolute_overlay <- function(...) {
    # Computes the per-cell minimum absolute value across a raster stack, propagating NoData if any input cell is NoData.
    run_tool("min_absolute_overlay", list(...))
  }
  session$min_dist_classification <- function(...) {
    # Performs a supervised minimum-distance classification on multi-spectral rasters using polygon training data.
    run_tool("min_dist_classification", list(...))
  }
  session$min_downslope_elev_change <- function(...) {
    # Calculates the minimum non-negative elevation drop to neighbouring cells.
    run_tool("min_downslope_elev_change", list(...))
  }
  session$min_max_contrast_stretch <- function(...) {
    # Linearly stretches values between user-specified minimum and maximum.
    run_tool("min_max_contrast_stretch", list(...))
  }
  session$min_overlay <- function(...) {
    # Computes the per-cell minimum across a raster stack, propagating NoData if any input cell is NoData.
    run_tool("min_overlay", list(...))
  }
  session$minimal_curvature <- function(...) {
    # Calculates minimal (minimum principal) curvature from a DEM.
    run_tool("minimal_curvature", list(...))
  }
  session$minimal_dispersion_flow_algorithm <- function(...) {
    # Generates MDFA flow-direction and flow-accumulation rasters from a DEM.
    run_tool("minimal_dispersion_flow_algorithm", list(...))
  }
  session$minimum_bounding_box <- function(...) {
    # Calculates oriented minimum bounding boxes around individual features or the entire layer.
    run_tool("minimum_bounding_box", list(...))
  }
  session$minimum_bounding_circle <- function(...) {
    # Calculates minimum enclosing circles around individual features or the entire layer.
    run_tool("minimum_bounding_circle", list(...))
  }
  session$minimum_bounding_envelope <- function(...) {
    # Calculates axis-aligned minimum bounding envelopes around individual features or the entire layer.
    run_tool("minimum_bounding_envelope", list(...))
  }
  session$minimum_convex_hull <- function(...) {
    # Creates convex hull polygons around individual features or the full input layer.
    run_tool("minimum_convex_hull", list(...))
  }
  session$minimum_filter <- function(...) {
    # Computes a moving-window minimum for each raster cell.
    run_tool("minimum_filter", list(...))
  }
  session$modified_k_means_clustering <- function(...) {
    # Performs modified k-means clustering with centroid merging based on a user-defined merge distance.
    run_tool("modified_k_means_clustering", list(...))
  }
  session$modified_shepard_interpolation <- function(...) {
    # Interpolates a raster from point samples using locally weighted modified-Shepard blending.
    run_tool("modified_shepard_interpolation", list(...))
  }
  session$modify_lidar <- function(...) {
    # Applies assignment expressions to modify LiDAR point attributes.
    run_tool("modify_lidar", list(...))
  }
  session$modify_nodata_value <- function(...) {
    # Changes the raster nodata value and rewrites existing nodata cells to the new value.
    run_tool("modify_nodata_value", list(...))
  }
  session$modulo <- function(...) {
    # Computes the remainder of dividing the first raster by the second on a cell-by-cell basis.
    run_tool("modulo", list(...))
  }
  session$mosaic <- function(...) {
    # Mosaics two or more rasters into a new output raster using nearest-neighbour, bilinear, or cubic resampling.
    run_tool("mosaic", list(...))
  }
  session$mosaic_with_feathering <- function(...) {
    # Mosaics two rasters and feather-blends overlapping cells using edge-distance weights.
    run_tool("mosaic_with_feathering", list(...))
  }
  session$multidirectional_hillshade <- function(...) {
    # Produces weighted multi-azimuth shaded-relief.
    run_tool("multidirectional_hillshade", list(...))
  }
  session$multimodal_od_cost_matrix <- function(...) {
    # Computes batched multimodal OD costs and mode summaries between origin and destination point sets.
    run_tool("multimodal_od_cost_matrix", list(...))
  }
  session$multimodal_routes_from_od <- function(...) {
    # Builds route geometries for multimodal origin-destination point pairs with per-route mode summaries.
    run_tool("multimodal_routes_from_od", list(...))
  }
  session$multimodal_shortest_path <- function(...) {
    # Finds a mode-aware shortest path over a line network with configurable transfer penalties.
    run_tool("multimodal_shortest_path", list(...))
  }
  session$multipart_to_singlepart <- function(...) {
    # Converts a vector containing multi-part features into one with only single-part features.
    run_tool("multipart_to_singlepart", list(...))
  }
  session$multiply <- function(...) {
    # Multiplies two rasters on a cell-by-cell basis.
    run_tool("multiply", list(...))
  }
  session$multiply_overlay <- function(...) {
    # Computes the per-cell product across a raster stack, propagating NoData if any input cell is NoData.
    run_tool("multiply_overlay", list(...))
  }
  session$multiscale_curvatures <- function(...) {
    # Calculates multiscale curvatures and curvature-based indices from a DEM.
    run_tool("multiscale_curvatures", list(...))
  }
  session$multiscale_elevated_index <- function(...) {
    # Calculates multiscale elevated-index (MsEI) and key-scale rasters using Gaussian scale-space residuals.
    run_tool("multiscale_elevated_index", list(...))
  }
  session$multiscale_elevation_percentile <- function(...) {
    # Calculates the most extreme local elevation percentile across a range of neighbourhood scales.
    run_tool("multiscale_elevation_percentile", list(...))
  }
  session$multiscale_low_lying_index <- function(...) {
    # Calculates multiscale low-lying-index (MsLLI) and key-scale rasters using Gaussian scale-space residuals.
    run_tool("multiscale_low_lying_index", list(...))
  }
  session$multiscale_roughness <- function(...) {
    # Calculates surface roughness over a range of neighbourhood scales.
    run_tool("multiscale_roughness", list(...))
  }
  session$multiscale_roughness_signature <- function(...) {
    # Calculates multiscale roughness signatures for input point sites and writes an HTML report.
    run_tool("multiscale_roughness_signature", list(...))
  }
  session$multiscale_std_dev_normals <- function(...) {
    # Calculates maximum spherical standard deviation of surface normals over a nonlinearly sampled range of scales.
    run_tool("multiscale_std_dev_normals", list(...))
  }
  session$multiscale_std_dev_normals_signature <- function(...) {
    # Calculates spherical-standard-deviation scale signatures for input point sites and writes an HTML report.
    run_tool("multiscale_std_dev_normals_signature", list(...))
  }
  session$multiscale_topographic_position_image <- function(...) {
    # Creates a packed RGB multiscale topographic-position image from local, meso, and broad DEVmax rasters.
    run_tool("multiscale_topographic_position_image", list(...))
  }
  session$narrowness_index <- function(...) {
    # Computes narrowness index (perimeter / sqrt(area)) for polygon features.
    run_tool("narrowness_index", list(...))
  }
  session$natural_neighbour_interpolation <- function(...) {
    # Interpolates a raster from point samples using a Delaunay-neighbour weighted scheme.
    run_tool("natural_neighbour_interpolation", list(...))
  }
  session$near <- function(...) {
    # Finds the nearest feature in a near layer and writes NEAR_FID and NEAR_DIST attributes.
    run_tool("near", list(...))
  }
  session$nearest_neighbour_interpolation <- function(...) {
    # Interpolates a raster from point samples by assigning each cell the nearest sample value.
    run_tool("nearest_neighbour_interpolation", list(...))
  }
  session$negate <- function(...) {
    # Negates each non-nodata raster cell value.
    run_tool("negate", list(...))
  }
  session$network_accessibility_metrics <- function(...) {
    # Computes accessibility indices for origin points based on reachability to destinations with optional impedance cutoffs and decay functions.
    run_tool("network_accessibility_metrics", list(...))
  }
  session$network_centrality_metrics <- function(...) {
    # Computes baseline degree, closeness, and betweenness centrality metrics for network nodes.
    run_tool("network_centrality_metrics", list(...))
  }
  session$network_connected_components <- function(...) {
    # Assigns a connected-component ID to each line feature in a network.
    run_tool("network_connected_components", list(...))
  }
  session$network_node_degree <- function(...) {
    # Extracts network nodes from line features and computes node degree and node type.
    run_tool("network_node_degree", list(...))
  }
  session$network_od_cost_matrix <- function(...) {
    # Computes origin-destination shortest-path costs over a line network and writes a CSV matrix.
    run_tool("network_od_cost_matrix", list(...))
  }
  session$network_routes_from_od <- function(...) {
    # Builds route geometries for origin-destination point pairs over a line network.
    run_tool("network_routes_from_od", list(...))
  }
  session$network_service_area <- function(...) {
    # Computes reachable network nodes from origin points within a maximum network cost.
    run_tool("network_service_area", list(...))
  }
  session$network_topology_audit <- function(...) {
    # Audits a line network for topology anomalies—disconnected components, dead ends, and degree anomalies—that cause routing failures.
    run_tool("network_topology_audit", list(...))
  }
  session$new_raster_from_base_raster <- function(...) {
    # Creates a new raster using the extent, dimensions, and CRS of a base raster.
    run_tool("new_raster_from_base_raster", list(...))
  }
  session$new_raster_from_base_vector <- function(...) {
    # Creates a new raster from a base vector extent and cell size, filled with an optional value.
    run_tool("new_raster_from_base_vector", list(...))
  }
  session$nibble <- function(...) {
    # Fills background regions using nearest-neighbour allocation.
    run_tool("nibble", list(...))
  }
  session$nnd_classification <- function(...) {
    # Performs nearest-normalized-distance classification with optional outlier rejection.
    run_tool("nnd_classification", list(...))
  }
  session$non_local_means_filter <- function(...) {
    # Performs non-local means denoising using patch similarity weighting.
    run_tool("non_local_means_filter", list(...))
  }
  session$normal_vectors <- function(...) {
    # Estimates local point-cloud normals and stores them in point normals and RGB values.
    run_tool("normal_vectors", list(...))
  }
  session$normalize_lidar <- function(...) {
    # Normalizes LiDAR z-values using a raster DTM so elevations become height above ground.
    run_tool("normalize_lidar", list(...))
  }
  session$normalized_difference_index <- function(...) {
    # Computes (band1 - band2) / (band1 + band2) from a multiband raster.
    run_tool("normalized_difference_index", list(...))
  }
  session$not_equal_to <- function(...) {
    # Tests whether two rasters are not equal on a cell-by-cell basis.
    run_tool("not_equal_to", list(...))
  }
  session$num_downslope_neighbours <- function(...) {
    # Counts the number of 8-neighbour cells lower than each DEM cell.
    run_tool("num_downslope_neighbours", list(...))
  }
  session$num_inflowing_neighbours <- function(...) {
    # Counts the number of inflowing D8 neighbours for each DEM cell.
    run_tool("num_inflowing_neighbours", list(...))
  }
  session$num_upslope_neighbours <- function(...) {
    # Counts the number of 8-neighbour cells higher than each DEM cell.
    run_tool("num_upslope_neighbours", list(...))
  }
  session$od_sensitivity_analysis <- function(...) {
    # Computes OD shortest-path costs with impedance perturbations and outputs sensitivity statistics via Monte Carlo sampling.
    run_tool("od_sensitivity_analysis", list(...))
  }
  session$olympic_filter <- function(...) {
    # Performs Olympic smoothing by averaging local values excluding min and max.
    run_tool("olympic_filter", list(...))
  }
  session$opening <- function(...) {
    # Performs a morphological opening operation using a rectangular structuring element.
    run_tool("opening", list(...))
  }
  session$openness <- function(...) {
    # Calculates Yokoyama et al. (2002) topographic openness from an input DEM. Returns positive (convex) and negative (concave) openness rasters.
    run_tool("openness", list(...))
  }
  session$otsu_thresholding <- function(...) {
    # Applies Otsu's automatic thresholding to create a binary raster.
    run_tool("otsu_thresholding", list(...))
  }
  session$paired_sample_t_test <- function(...) {
    # Performs a paired-sample t-test on two rasters using paired valid cells.
    run_tool("paired_sample_t_test", list(...))
  }
  session$panchromatic_sharpening <- function(...) {
    # Fuses multispectral and panchromatic rasters using Brovey or IHS methods.
    run_tool("panchromatic_sharpening", list(...))
  }
  session$parallelepiped_classification <- function(...) {
    # Performs a supervised parallelepiped classification on multi-spectral rasters using polygon training data.
    run_tool("parallelepiped_classification", list(...))
  }
  session$patch_orientation <- function(...) {
    # Calculates polygon orientation (degrees from north) using reduced major axis regression and appends ORIENT.
    run_tool("patch_orientation", list(...))
  }
  session$pennock_landform_classification <- function(...) {
    # Classifies landform elements into seven Pennock et al. (1987) terrain classes.
    run_tool("pennock_landform_classification", list(...))
  }
  session$percent_elev_range <- function(...) {
    # Calculates local topographic position as percent of neighbourhood elevation range.
    run_tool("percent_elev_range", list(...))
  }
  session$percent_equal_to <- function(...) {
    # Computes the fraction of rasters in a stack whose values equal the comparison raster at each cell.
    run_tool("percent_equal_to", list(...))
  }
  session$percent_greater_than <- function(...) {
    # Computes the fraction of rasters in a stack whose values are greater than the comparison raster at each cell.
    run_tool("percent_greater_than", list(...))
  }
  session$percent_less_than <- function(...) {
    # Computes the fraction of rasters in a stack whose values are less than the comparison raster at each cell.
    run_tool("percent_less_than", list(...))
  }
  session$percentage_contrast_stretch <- function(...) {
    # Performs linear contrast stretch with percentile clipping.
    run_tool("percentage_contrast_stretch", list(...))
  }
  session$percentile_filter <- function(...) {
    # Computes center-cell percentile rank in a moving window.
    run_tool("percentile_filter", list(...))
  }
  session$perimeter_area_ratio <- function(...) {
    # Calculates polygon perimeter/area ratio and appends P_A_RATIO.
    run_tool("perimeter_area_ratio", list(...))
  }
  session$phi_coefficient <- function(...) {
    # Performs binary classification agreement assessment using the phi coefficient.
    run_tool("phi_coefficient", list(...))
  }
  session$pick_from_list <- function(...) {
    # Selects per-cell values from a raster stack using a zero-based position raster.
    run_tool("pick_from_list", list(...))
  }
  session$piecewise_contrast_stretch <- function(...) {
    # Performs piecewise linear contrast stretching using user-specified breakpoints.
    run_tool("piecewise_contrast_stretch", list(...))
  }
  session$plan_curvature <- function(...) {
    # Calculates plan (contour) curvature from a DEM.
    run_tool("plan_curvature", list(...))
  }
  session$points_along_lines <- function(...) {
    # Creates regularly spaced point features along input line geometries.
    run_tool("points_along_lines", list(...))
  }
  session$polygon_area <- function(...) {
    # Calculates polygon area and appends an AREA attribute field.
    run_tool("polygon_area", list(...))
  }
  session$polygon_long_axis <- function(...) {
    # Maps the long axis of each polygon feature's minimum bounding box as line output.
    run_tool("polygon_long_axis", list(...))
  }
  session$polygon_perimeter <- function(...) {
    # Calculates polygon perimeter and appends a PERIMETER attribute field.
    run_tool("polygon_perimeter", list(...))
  }
  session$polygon_short_axis <- function(...) {
    # Maps the short axis of each polygon feature's minimum bounding box as line output.
    run_tool("polygon_short_axis", list(...))
  }
  session$polygonize <- function(...) {
    # Creates polygons from closed input linework rings.
    run_tool("polygonize", list(...))
  }
  session$polygons_to_lines <- function(...) {
    # Converts polygon and multipolygon features into linework tracing their boundaries.
    run_tool("polygons_to_lines", list(...))
  }
  session$power <- function(...) {
    # Raises the first raster to the power of the second on a cell-by-cell basis.
    run_tool("power", list(...))
  }
  session$prewitt_filter <- function(...) {
    # Performs Prewitt edge detection.
    run_tool("prewitt_filter", list(...))
  }
  session$principal_component_analysis <- function(...) {
    # Performs PCA on a stack of rasters, returning component images and a JSON report.
    run_tool("principal_component_analysis", list(...))
  }
  session$principal_curvature_direction <- function(...) {
    # Calculates the principal curvature direction angle (degrees).
    run_tool("principal_curvature_direction", list(...))
  }
  session$print_geotiff_tags <- function(...) {
    # Produces a text report describing TIFF/GeoTIFF tags and key metadata for an input GeoTIFF-family raster.
    run_tool("print_geotiff_tags", list(...))
  }
  session$profile <- function(...) {
    # Creates an HTML elevation profile plot for one or more input polyline features sampled from a surface raster.
    run_tool("profile", list(...))
  }
  session$profile_curvature <- function(...) {
    # Calculates profile curvature from a DEM.
    run_tool("profile_curvature", list(...))
  }
  session$prune_vector_streams <- function(...) {
    # Prunes vector stream network based on Shreve magnitude.
    run_tool("prune_vector_streams", list(...))
  }
  session$qin_flow_accumulation <- function(...) {
    # Calculates Qin MFD flow accumulation from a DEM.
    run_tool("qin_flow_accumulation", list(...))
  }
  session$quantiles <- function(...) {
    # Transforms raster values into quantile classes.
    run_tool("quantiles", list(...))
  }
  session$quinn_flow_accumulation <- function(...) {
    # Calculates Quinn MFD flow accumulation from a DEM.
    run_tool("quinn_flow_accumulation", list(...))
  }
  session$radial_basis_function_interpolation <- function(...) {
    # Interpolates a raster from point samples using local radial-basis similarity weighting.
    run_tool("radial_basis_function_interpolation", list(...))
  }
  session$radius_of_gyration <- function(...) {
    # Computes per-patch radius of gyration and maps values back to patch cells.
    run_tool("radius_of_gyration", list(...))
  }
  session$raise_walls <- function(...) {
    # Raises DEM elevations along wall vectors and optionally breaches selected crossings.
    run_tool("raise_walls", list(...))
  }
  session$random_field <- function(...) {
    # Creates a raster containing standard normal random values.
    run_tool("random_field", list(...))
  }
  session$random_forest_classification <- function(...) {
    # Performs supervised random forest classification on multi-band input rasters.
    run_tool("random_forest_classification", list(...))
  }
  session$random_forest_classification_fit <- function(...) {
    # Fits a random forest classification model and returns serialized model bytes.
    run_tool("random_forest_classification_fit", list(...))
  }
  session$random_forest_classification_predict <- function(...) {
    # Applies a serialized random forest classification model to multi-band predictors.
    run_tool("random_forest_classification_predict", list(...))
  }
  session$random_forest_regression <- function(...) {
    # Performs supervised random forest regression on multi-band input rasters.
    run_tool("random_forest_regression", list(...))
  }
  session$random_forest_regression_fit <- function(...) {
    # Fits a random forest regression model and returns serialized model bytes.
    run_tool("random_forest_regression_fit", list(...))
  }
  session$random_forest_regression_predict <- function(...) {
    # Applies a serialized random forest regression model to multi-band predictors.
    run_tool("random_forest_regression_predict", list(...))
  }
  session$random_points_in_polygon <- function(...) {
    # Generates random points uniformly within input polygon geometries.
    run_tool("random_points_in_polygon", list(...))
  }
  session$random_sample <- function(...) {
    # Creates a raster containing randomly located sample cells with unique IDs.
    run_tool("random_sample", list(...))
  }
  session$range_filter <- function(...) {
    # Computes a moving-window range (max-min) for each raster cell.
    run_tool("range_filter", list(...))
  }
  session$raster_area <- function(...) {
    # Estimates per-class raster polygon area in grid-cell or map units and writes class totals to each class cell.
    run_tool("raster_area", list(...))
  }
  session$raster_calculator <- function(...) {
    # Evaluates a mathematical expression on a list of input rasters cell-by-cell.
    run_tool("raster_calculator", list(...))
  }
  session$raster_cell_assignment <- function(...) {
    # Creates a raster derived from a base raster assigning row, column, x, or y values to each cell.
    run_tool("raster_cell_assignment", list(...))
  }
  session$raster_histogram <- function(...) {
    # Builds a fixed-bin histogram for valid raster cells.
    run_tool("raster_histogram", list(...))
  }
  session$raster_perimeter <- function(...) {
    # Estimates per-class raster polygon perimeter using an anti-aliasing lookup table and writes class totals to each class cell.
    run_tool("raster_perimeter", list(...))
  }
  session$raster_streams_to_vector <- function(...) {
    # Converts raster stream network to vector.
    run_tool("raster_streams_to_vector", list(...))
  }
  session$raster_summary_stats <- function(...) {
    # Computes basic summary statistics for valid raster cells.
    run_tool("raster_summary_stats", list(...))
  }
  session$raster_to_vector_lines <- function(...) {
    # Converts non-zero, non-nodata raster line cells into polyline vector features.
    run_tool("raster_to_vector_lines", list(...))
  }
  session$raster_to_vector_points <- function(...) {
    # Converts non-zero, non-nodata cells in a raster into point features located at cell centres.
    run_tool("raster_to_vector_points", list(...))
  }
  session$raster_to_vector_polygons <- function(...) {
    # Converts non-zero, non-nodata raster regions into polygon vector features with FID and VALUE attributes.
    run_tool("raster_to_vector_polygons", list(...))
  }
  session$rasterize_streams <- function(...) {
    # Rasterizes vector stream network.
    run_tool("rasterize_streams", list(...))
  }
  session$reciprocal <- function(...) {
    # Computes the reciprocal (1/x) of each raster cell.
    run_tool("reciprocal", list(...))
  }
  session$reclass <- function(...) {
    # Reclassifies raster values using either ranges or exact assignment pairs.
    run_tool("reclass", list(...))
  }
  session$reclass_equal_interval <- function(...) {
    # Reclassifies raster values into equal-width intervals over an optional value range.
    run_tool("reclass_equal_interval", list(...))
  }
  session$recover_flightline_info <- function(...) {
    # Infers flightlines from GPS-time gaps and writes identifiers to point source ID, user data, and/or RGB.
    run_tool("recover_flightline_info", list(...))
  }
  session$rectangular_grid_from_raster_base <- function(...) {
    # Creates a rectangular polygon grid covering a raster extent.
    run_tool("rectangular_grid_from_raster_base", list(...))
  }
  session$rectangular_grid_from_vector_base <- function(...) {
    # Creates a rectangular polygon grid covering a vector-layer bounding extent.
    run_tool("rectangular_grid_from_vector_base", list(...))
  }
  session$reinitialize_attribute_table <- function(...) {
    # Creates a copy of a vector layer with only a regenerated FID attribute.
    run_tool("reinitialize_attribute_table", list(...))
  }
  session$related_circumscribing_circle <- function(...) {
    # Calculates 1 - (polygon area / smallest circumscribing circle area) and appends RC_CIRCLE.
    run_tool("related_circumscribing_circle", list(...))
  }
  session$relative_aspect <- function(...) {
    # Calculates terrain aspect relative to a user-specified azimuth (0 to 180 degrees).
    run_tool("relative_aspect", list(...))
  }
  session$relative_stream_power_index <- function(...) {
    # Calculates the relative stream power index from specific catchment area and slope.
    run_tool("relative_stream_power_index", list(...))
  }
  session$relative_topographic_position <- function(...) {
    # Calculates RTP using neighbourhood min, mean, and max elevation values.
    run_tool("relative_topographic_position", list(...))
  }
  session$remove_duplicates <- function(...) {
    # Removes duplicate LiDAR points using x/y and optionally z coordinates.
    run_tool("remove_duplicates", list(...))
  }
  session$remove_off_terrain_objects <- function(...) {
    # Removes steep off-terrain objects from DEMs using white top-hat normalization, slope-constrained region growing, and local interpolation.
    run_tool("remove_off_terrain_objects", list(...))
  }
  session$remove_polygon_holes <- function(...) {
    # Removes interior rings from polygon features while preserving attributes.
    run_tool("remove_polygon_holes", list(...))
  }
  session$remove_raster_polygon_holes <- function(...) {
    # Removes interior background holes (0 or nodata regions enclosed by foreground) from raster polygons.
    run_tool("remove_raster_polygon_holes", list(...))
  }
  session$remove_short_streams <- function(...) {
    # Removes stream links shorter than minimum length.
    run_tool("remove_short_streams", list(...))
  }
  session$remove_spurs <- function(...) {
    # Removes short spur artifacts from binary raster features by iterative pruning.
    run_tool("remove_spurs", list(...))
  }
  session$rename_field <- function(...) {
    # Renames an attribute field in a vector layer.
    run_tool("rename_field", list(...))
  }
  session$repair_stream_vector_topology <- function(...) {
    # Repairs topology of vector stream network.
    run_tool("repair_stream_vector_topology", list(...))
  }
  session$reproject_vector <- function(...) {
    # Reprojects an input vector layer to a destination EPSG code.
    run_tool("reproject_vector", list(...))
  }
  session$resample <- function(...) {
    # Resamples one or more input rasters to a base raster grid or to a user-defined output cell size.
    run_tool("resample", list(...))
  }
  session$rescale_value_range <- function(...) {
    # Linearly rescales raster values into a target range.
    run_tool("rescale_value_range", list(...))
  }
  session$rgb_to_ihs <- function(...) {
    # Transforms red, green, blue band rasters (or a packed composite) to intensity, hue, and saturation components.
    run_tool("rgb_to_ihs", list(...))
  }
  session$rho8_flow_accum <- function(...) {
    # Calculates Rho8 flow accumulation from a DEM or Rho8 pointer raster.
    run_tool("rho8_flow_accum", list(...))
  }
  session$rho8_pointer <- function(...) {
    # Generates a Rho8 stochastic single-flow-direction pointer raster from a DEM.
    run_tool("rho8_pointer", list(...))
  }
  session$ridge_and_valley_vectors <- function(...) {
    # Extracts ridge and valley centreline vectors from a DEM.
    run_tool("ridge_and_valley_vectors", list(...))
  }
  session$ring_curvature <- function(...) {
    # Calculates ring curvature (squared flow-line twisting) from a DEM.
    run_tool("ring_curvature", list(...))
  }
  session$river_centerlines <- function(...) {
    # Extracts river centerlines from water raster using medial axis.
    run_tool("river_centerlines", list(...))
  }
  session$roberts_cross_filter <- function(...) {
    # Performs Roberts Cross edge detection.
    run_tool("roberts_cross_filter", list(...))
  }
  session$root_mean_square_error <- function(...) {
    # Calculates RMSE and related accuracy statistics between two rasters.
    run_tool("root_mean_square_error", list(...))
  }
  session$rotor <- function(...) {
    # Calculates the rotor (flow-line twisting) from a DEM.
    run_tool("rotor", list(...))
  }
  session$round <- function(...) {
    # Rounds each raster cell to the nearest integer.
    run_tool("round", list(...))
  }
  session$route_calibrate <- function(...) {
    # Calibrates route start/end measures from control points with known measures.
    run_tool("route_calibrate", list(...))
  }
  session$route_event_lines_from_layer <- function(...) {
    # Creates routed line events from an event vector layer using from/to measures.
    run_tool("route_event_lines_from_layer", list(...))
  }
  session$route_event_lines_from_table <- function(...) {
    # Creates routed line events from a CSV event table and a route layer using from/to measures.
    run_tool("route_event_lines_from_table", list(...))
  }
  session$route_event_merge <- function(...) {
    # Merges adjacent compatible route events.
    run_tool("route_event_merge", list(...))
  }
  session$route_event_overlay <- function(...) {
    # Overlays two route event layers by interval overlap.
    run_tool("route_event_overlay", list(...))
  }
  session$route_event_points_from_layer <- function(...) {
    # Creates routed point events from an event vector layer and a route layer.
    run_tool("route_event_points_from_layer", list(...))
  }
  session$route_event_points_from_table <- function(...) {
    # Creates routed point events from a CSV event table and a route layer.
    run_tool("route_event_points_from_table", list(...))
  }
  session$route_event_split <- function(...) {
    # Splits route events by per-route boundary measures.
    run_tool("route_event_split", list(...))
  }
  session$route_measure_qa <- function(...) {
    # Diagnoses route-event measure gaps, overlaps, non-monotonic sequences, and duplicate measures.
    run_tool("route_measure_qa", list(...))
  }
  session$route_recalibrate <- function(...) {
    # Recalibrates edited route measures from a reference route layer while preserving route measure continuity.
    run_tool("route_recalibrate", list(...))
  }
  session$ruggedness_index <- function(...) {
    # Calculates the terrain ruggedness index (TRI) after Riley et al. (1999).
    run_tool("ruggedness_index", list(...))
  }
  session$savitzky_golay_2d_filter <- function(...) {
    # Performs 2D Savitzky-Golay smoothing.
    run_tool("savitzky_golay_2d_filter", list(...))
  }
  session$scharr_filter <- function(...) {
    # Performs Scharr edge detection.
    run_tool("scharr_filter", list(...))
  }
  session$sediment_transport_index <- function(...) {
    # Calculates the sediment transport index (LS factor) from specific catchment area and slope.
    run_tool("sediment_transport_index", list(...))
  }
  session$select_by_location <- function(...) {
    # Extracts target features that satisfy a spatial relationship to query features.
    run_tool("select_by_location", list(...))
  }
  session$select_tiles_by_polygon <- function(...) {
    # Copies LiDAR tiles from an input directory to an output directory when tile sample points overlap polygon geometries.
    run_tool("select_tiles_by_polygon", list(...))
  }
  session$set_nodata_value <- function(...) {
    # Sets a raster nodata value and maps existing nodata cells to the specified background value.
    run_tool("set_nodata_value", list(...))
  }
  session$shape_complexity_index_raster <- function(...) {
    # Computes raster patch shape complexity from horizontal/vertical transition frequency normalized by patch span.
    run_tool("shape_complexity_index_raster", list(...))
  }
  session$shape_complexity_index_vector <- function(...) {
    # Computes shape complexity index for vector polygon features using normalized form factor.
    run_tool("shape_complexity_index_vector", list(...))
  }
  session$shape_index <- function(...) {
    # Calculates the shape index surface form descriptor from a DEM.
    run_tool("shape_index", list(...))
  }
  session$shortest_path_network <- function(...) {
    # Finds the shortest path between start and end coordinates over a line network.
    run_tool("shortest_path_network", list(...))
  }
  session$shreve_stream_magnitude <- function(...) {
    # Calculates Shreve stream magnitude.
    run_tool("shreve_stream_magnitude", list(...))
  }
  session$sieve <- function(...) {
    # Removes small isolated patches below a cell-count threshold.
    run_tool("sieve", list(...))
  }
  session$sigmoidal_contrast_stretch <- function(...) {
    # Performs sigmoidal contrast stretching using gain and cutoff.
    run_tool("sigmoidal_contrast_stretch", list(...))
  }
  session$simplify_features <- function(...) {
    # Simplifies vector geometries using Douglas-Peucker tolerance.
    run_tool("simplify_features", list(...))
  }
  session$sin <- function(...) {
    # Computes the sine of each raster cell value.
    run_tool("sin", list(...))
  }
  session$singlepart_to_multipart <- function(...) {
    # Merges single-part features into multi-part features, grouped by an optional categorical field.
    run_tool("singlepart_to_multipart", list(...))
  }
  session$sinh <- function(...) {
    # Computes the hyperbolic sine of each raster cell.
    run_tool("sinh", list(...))
  }
  session$sink <- function(...) {
    # Identifies cells that belong to topographic depressions in a DEM.
    run_tool("sink", list(...))
  }
  session$sky_view_factor <- function(...) {
    # Calculates the proportion of visible sky from a DEM/DSM.
    run_tool("sky_view_factor", list(...))
  }
  session$slope <- function(...) {
    # Calculates slope gradient from a DEM.
    run_tool("slope", list(...))
  }
  session$slope_vs_elev_plot <- function(...) {
    # Creates an HTML slope-vs-elevation analysis chart for one or more DEMs.
    run_tool("slope_vs_elev_plot", list(...))
  }
  session$smooth_vectors <- function(...) {
    # Smooths polyline or polygon vectors using a moving-average filter.
    run_tool("smooth_vectors", list(...))
  }
  session$snap_endnodes <- function(...) {
    # Snaps nearby polyline endpoints to a shared location within a tolerance.
    run_tool("snap_endnodes", list(...))
  }
  session$snap_pour_points <- function(...) {
    # Snaps pour points to the highest flow-accumulation cell within a search distance.
    run_tool("snap_pour_points", list(...))
  }
  session$sobel_filter <- function(...) {
    # Performs Sobel edge detection.
    run_tool("sobel_filter", list(...))
  }
  session$sort_lidar <- function(...) {
    # Sorts points by one or more LiDAR properties, with optional bin sizes per criterion.
    run_tool("sort_lidar", list(...))
  }
  session$spatial_join <- function(...) {
    # Joins attributes from a join layer onto target features using a spatial predicate.
    run_tool("spatial_join", list(...))
  }
  session$spherical_std_dev_of_normals <- function(...) {
    # Calculates spherical standard deviation of local surface normals.
    run_tool("spherical_std_dev_of_normals", list(...))
  }
  session$split_colour_composite <- function(...) {
    # Splits a packed RGB colour composite into separate red, green, and blue single-band rasters.
    run_tool("split_colour_composite", list(...))
  }
  session$split_lidar <- function(...) {
    # Splits LiDAR points into multiple output files based on a grouping criterion.
    run_tool("split_lidar", list(...))
  }
  session$split_vector_lines <- function(...) {
    # Splits each polyline feature into segments of a maximum specified length.
    run_tool("split_vector_lines", list(...))
  }
  session$split_with_lines <- function(...) {
    # Splits input polylines using intersection points from a split line layer.
    run_tool("split_with_lines", list(...))
  }
  session$sqrt <- function(...) {
    # Computes the square-root of each raster cell.
    run_tool("sqrt", list(...))
  }
  session$square <- function(...) {
    # Squares each raster cell value.
    run_tool("square", list(...))
  }
  session$standard_deviation_contrast_stretch <- function(...) {
    # Performs linear contrast stretch using mean plus/minus a standard deviation multiplier.
    run_tool("standard_deviation_contrast_stretch", list(...))
  }
  session$standard_deviation_filter <- function(...) {
    # Computes a moving-window standard deviation for each raster cell.
    run_tool("standard_deviation_filter", list(...))
  }
  session$standard_deviation_of_slope <- function(...) {
    # Calculates local standard deviation of slope as a terrain roughness metric.
    run_tool("standard_deviation_of_slope", list(...))
  }
  session$standard_deviation_overlay <- function(...) {
    # Computes the per-cell standard deviation across a raster stack, propagating NoData if any input cell is NoData.
    run_tool("standard_deviation_overlay", list(...))
  }
  session$stochastic_depression_analysis <- function(...) {
    # Runs Monte Carlo DEM perturbations and estimates depression-membership probability.
    run_tool("stochastic_depression_analysis", list(...))
  }
  session$strahler_order_basins <- function(...) {
    # Delineates watershed basins labelled by the Horton-Strahler order of their draining stream link.
    run_tool("strahler_order_basins", list(...))
  }
  session$strahler_stream_order <- function(...) {
    # Assigns Strahler stream order to stream cells.
    run_tool("strahler_stream_order", list(...))
  }
  session$stream_link_class <- function(...) {
    # Classifies stream links as interior, exterior, or source.
    run_tool("stream_link_class", list(...))
  }
  session$stream_link_identifier <- function(...) {
    # Assigns unique ID to each stream link.
    run_tool("stream_link_identifier", list(...))
  }
  session$stream_link_length <- function(...) {
    # Calculates total length for each stream link.
    run_tool("stream_link_length", list(...))
  }
  session$stream_link_slope <- function(...) {
    # Calculates average slope for each stream link.
    run_tool("stream_link_slope", list(...))
  }
  session$stream_slope_continuous <- function(...) {
    # Calculates slope value for each stream cell.
    run_tool("stream_slope_continuous", list(...))
  }
  session$subbasins <- function(...) {
    # Identifies the catchment area of each stream link (sub-basins) in a D8 stream network.
    run_tool("subbasins", list(...))
  }
  session$subtract <- function(...) {
    # Subtracts the second raster from the first on a cell-by-cell basis.
    run_tool("subtract", list(...))
  }
  session$sum_overlay <- function(...) {
    # Computes the per-cell sum across a raster stack, propagating NoData if any input cell is NoData.
    run_tool("sum_overlay", list(...))
  }
  session$surface_area_ratio <- function(...) {
    # Calculates the ratio of 3D surface area to planimetric area using the Jenness (2004) method.
    run_tool("surface_area_ratio", list(...))
  }
  session$svm_classification <- function(...) {
    # Performs supervised support-vector-machine classification on multi-band input rasters.
    run_tool("svm_classification", list(...))
  }
  session$svm_regression <- function(...) {
    # Performs supervised support-vector-machine regression on multi-band input rasters.
    run_tool("svm_regression", list(...))
  }
  session$symmetrical_difference <- function(...) {
    # Computes non-overlapping polygon regions from input and overlay layers.
    run_tool("symmetrical_difference", list(...))
  }
  session$tan <- function(...) {
    # Computes the tangent of each raster cell value.
    run_tool("tan", list(...))
  }
  session$tangential_curvature <- function(...) {
    # Calculates tangential curvature from a DEM.
    run_tool("tangential_curvature", list(...))
  }
  session$tanh <- function(...) {
    # Computes the hyperbolic tangent of each raster cell.
    run_tool("tanh", list(...))
  }
  session$thicken_raster_line <- function(...) {
    # Thickens diagonal raster line segments to prevent diagonal leak-through.
    run_tool("thicken_raster_line", list(...))
  }
  session$time_in_daylight <- function(...) {
    # Calculates the proportion of daytime each cell is illuminated (not in terrain/object shadow).
    run_tool("time_in_daylight", list(...))
  }
  session$tin_interpolation <- function(...) {
    # Interpolates a raster from point samples using Delaunay triangulation and planar interpolation within each triangle.
    run_tool("tin_interpolation", list(...))
  }
  session$to_degrees <- function(...) {
    # Converts each raster cell from radians to degrees.
    run_tool("to_degrees", list(...))
  }
  session$to_radians <- function(...) {
    # Converts each raster cell from degrees to radians.
    run_tool("to_radians", list(...))
  }
  session$tophat_transform <- function(...) {
    # Performs a white or black morphological top-hat transform.
    run_tool("tophat_transform", list(...))
  }
  session$topological_stream_order <- function(...) {
    # Assigns topological stream order based on link count.
    run_tool("topological_stream_order", list(...))
  }
  session$topology_rule_autofix <- function(...) {
    # Automatically applies safe, auditable fixes to topology violations detected by topology_rule_validate.
    run_tool("topology_rule_autofix", list(...))
  }
  session$topology_rule_validate <- function(...) {
    # Validates vector topology against rule-set checks (self-intersection, overlap, gaps, dangles, point coverage, endpoint snapping) and emits feature-level violations.
    run_tool("topology_rule_validate", list(...))
  }
  session$topology_validation_report <- function(...) {
    # Audits a vector layer for topology issues and writes a per-feature CSV report.
    run_tool("topology_validation_report", list(...))
  }
  session$total_curvature <- function(...) {
    # Calculates total curvature from a DEM.
    run_tool("total_curvature", list(...))
  }
  session$total_filter <- function(...) {
    # Computes a moving-window total for each raster cell.
    run_tool("total_filter", list(...))
  }
  session$trace_downslope_flowpaths <- function(...) {
    # Marks D8 flowpaths initiated from seed points until no-flow or grid edge.
    run_tool("trace_downslope_flowpaths", list(...))
  }
  session$travelling_salesman_problem <- function(...) {
    # Finds approximate solutions to the travelling salesman problem (TSP) using 2-opt heuristics. Given a set of point locations, identifies the shortest route connecting all points.
    run_tool("travelling_salesman_problem", list(...))
  }
  session$trend_surface <- function(...) {
    # Fits a polynomial trend surface to a raster using least-squares regression.
    run_tool("trend_surface", list(...))
  }
  session$trend_surface_vector_points <- function(...) {
    # Fits a polynomial trend surface to vector point data using least-squares regression.
    run_tool("trend_surface_vector_points", list(...))
  }
  session$tributary_identifier <- function(...) {
    # Assigns unique ID to each tributary.
    run_tool("tributary_identifier", list(...))
  }
  session$truncate <- function(...) {
    # Truncates each raster cell value to its integer part.
    run_tool("truncate", list(...))
  }
  session$turning_bands_simulation <- function(...) {
    # Creates a spatially-autocorrelated random field using the turning bands algorithm.
    run_tool("turning_bands_simulation", list(...))
  }
  session$two_sample_ks_test <- function(...) {
    # Performs a two-sample Kolmogorov-Smirnov test on two raster value distributions.
    run_tool("two_sample_ks_test", list(...))
  }
  session$union <- function(...) {
    # Dissolves combined input and overlay polygons into a unified polygon coverage.
    run_tool("union", list(...))
  }
  session$unnest_basins <- function(...) {
    # Creates one basin raster per pour-point nesting level from a D8 pointer grid.
    run_tool("unnest_basins", list(...))
  }
  session$unsharp_masking <- function(...) {
    # Performs edge-enhancing unsharp masking.
    run_tool("unsharp_masking", list(...))
  }
  session$unsphericity <- function(...) {
    # Calculates the unsphericity curvature (half the difference of principal curvatures) from a DEM.
    run_tool("unsphericity", list(...))
  }
  session$update_nodata_cells <- function(...) {
    # Assigns NoData cells in input1 from corresponding valid cells in input2.
    run_tool("update_nodata_cells", list(...))
  }
  session$upslope_depression_storage <- function(...) {
    # Maps mean upslope depression-storage depth by routing depression depth over a conditioned DEM.
    run_tool("upslope_depression_storage", list(...))
  }
  session$user_defined_weights_filter <- function(...) {
    # Applies a user-defined convolution kernel.
    run_tool("user_defined_weights_filter", list(...))
  }
  session$vector_hex_binning <- function(...) {
    # Aggregates point features into hexagonal bins, counting points per hex cell.
    run_tool("vector_hex_binning", list(...))
  }
  session$vector_lines_to_raster <- function(...) {
    # Rasterizes line and polygon boundary geometries to a raster grid.
    run_tool("vector_lines_to_raster", list(...))
  }
  session$vector_points_to_raster <- function(...) {
    # Rasterizes point or multipoint vectors to a grid using a selected assignment operation.
    run_tool("vector_points_to_raster", list(...))
  }
  session$vector_polygons_to_raster <- function(...) {
    # Rasterizes polygon vectors to a grid, supporting attribute-driven burn values.
    run_tool("vector_polygons_to_raster", list(...))
  }
  session$vector_stream_network_analysis <- function(...) {
    # Comprehensive vector stream network analysis.
    run_tool("vector_stream_network_analysis", list(...))
  }
  session$vector_summary_statistics <- function(...) {
    # Computes grouped summary statistics for a numeric field and writes the result to CSV.
    run_tool("vector_summary_statistics", list(...))
  }
  session$vehicle_routing_cvrp <- function(...) {
    # Builds capacity-constrained delivery routes from depot and stop points using deterministic greedy construction with optional local optimization.
    run_tool("vehicle_routing_cvrp", list(...))
  }
  session$vehicle_routing_pickup_delivery <- function(...) {
    # Builds paired pickup-delivery routes with precedence and capacity constraints using a deterministic nearest-neighbour baseline.
    run_tool("vehicle_routing_pickup_delivery", list(...))
  }
  session$vehicle_routing_vrptw <- function(...) {
    # Builds capacity-constrained routes with time-window diagnostics using deterministic feasible-candidate scoring with optional nearest-neighbour baseline behavior.
    run_tool("vehicle_routing_vrptw", list(...))
  }
  session$vertical_excess_curvature <- function(...) {
    # Calculates vertical excess curvature from a DEM.
    run_tool("vertical_excess_curvature", list(...))
  }
  session$viewshed <- function(...) {
    # Computes station visibility counts from point stations over a DEM.
    run_tool("viewshed", list(...))
  }
  session$visibility_index <- function(...) {
    # Calculates a topography-based visibility index from sampled viewsheds.
    run_tool("visibility_index", list(...))
  }
  session$voronoi_diagram <- function(...) {
    # Creates Voronoi (Thiessen) polygons from input point locations.
    run_tool("voronoi_diagram", list(...))
  }
  session$watershed <- function(...) {
    # Delineates watersheds from a D8 pointer and vector pour points.
    run_tool("watershed", list(...))
  }
  session$watershed_from_raster_pour_points <- function(...) {
    # Delineates watersheds from a D8 pointer and a raster of pour-point outlet IDs.
    run_tool("watershed_from_raster_pour_points", list(...))
  }
  session$weighted_overlay <- function(...) {
    # Combines factor rasters using normalized weights, optional cost flags, and optional binary constraints.
    run_tool("weighted_overlay", list(...))
  }
  session$weighted_sum <- function(...) {
    # Computes a weighted sum across a raster stack after normalizing weights to sum to one.
    run_tool("weighted_sum", list(...))
  }
  session$wetness_index <- function(...) {
    # Calculates the topographic wetness index ln(SCA / tan(slope)).
    run_tool("wetness_index", list(...))
  }
  session$wiener_filter <- function(...) {
    # Performs adaptive Wiener denoising using local mean and variance.
    run_tool("wiener_filter", list(...))
  }
  session$wilcoxon_signed_rank_test <- function(...) {
    # Performs a Wilcoxon signed-rank test on paired raster differences.
    run_tool("wilcoxon_signed_rank_test", list(...))
  }
  session$write_function_memory_insertion <- function(...) {
    # Creates a packed RGB change-visualization composite from two or three single-band dates.
    run_tool("write_function_memory_insertion", list(...))
  }
  session$z_scores <- function(...) {
    # Standardizes raster values to z-scores using global mean and standard deviation.
    run_tool("z_scores", list(...))
  }
  session$zonal_statistics <- function(...) {
    # Summarises the values of a data raster within zones defined by a feature raster.
    run_tool("zonal_statistics", list(...))
  }

  session
}

wbw_run_tool <- function(tool_id, args = list()) {
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$run_tool(tool_id, args)
}

abs <- function(...) {
  # Calculates the absolute value of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$abs(...)
}

accumulation_curvature <- function(...) {
  # Calculates accumulation curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$accumulation_curvature(...)
}

adaptive_filter <- function(...) {
  # Performs adaptive thresholded mean replacement based on local z-scores.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$adaptive_filter(...)
}

add <- function(...) {
  # Adds two rasters on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$add(...)
}

add_field <- function(...) {
  # Adds a new attribute field with an optional default value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$add_field(...)
}

add_geometry_attributes <- function(...) {
  # Adds area, length, perimeter, and centroid attributes to vector features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$add_geometry_attributes(...)
}

add_point_coordinates_to_table <- function(...) {
  # Copies a point layer and appends XCOORD and YCOORD attribute fields.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$add_point_coordinates_to_table(...)
}

aggregate_raster <- function(...) {
  # Reduces raster resolution by aggregating blocks using mean, sum, min, max, or range.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$aggregate_raster(...)
}

anisotropic_diffusion_filter <- function(...) {
  # Performs Perona-Malik edge-preserving anisotropic diffusion smoothing.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$anisotropic_diffusion_filter(...)
}

anova <- function(...) {
  # Performs one-way ANOVA on raster values grouped by class raster categories.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$anova(...)
}

arccos <- function(...) {
  # Computes the inverse cosine (arccos) of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$arccos(...)
}

arcosh <- function(...) {
  # Computes the inverse hyperbolic cosine of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$arcosh(...)
}

arcsin <- function(...) {
  # Computes the inverse sine (arcsin) of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$arcsin(...)
}

arctan <- function(...) {
  # Computes the inverse tangent (arctan) of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$arctan(...)
}

arsinh <- function(...) {
  # Computes the inverse hyperbolic sine of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$arsinh(...)
}

artanh <- function(...) {
  # Computes the inverse hyperbolic tangent of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$artanh(...)
}

ascii_to_las <- function(...) {
  # Converts one or more ASCII LiDAR point files to LAS.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ascii_to_las(...)
}

aspect <- function(...) {
  # Calculates slope aspect in degrees clockwise from north.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$aspect(...)
}

atan2 <- function(...) {
  # Computes the four-quadrant inverse tangent using two rasters on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$atan2(...)
}

attribute_correlation <- function(...) {
  # Performs Pearson correlation analysis on numeric vector attribute fields.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$attribute_correlation(...)
}

attribute_histogram <- function(...) {
  # Creates a histogram for numeric field values in a vector attribute table.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$attribute_histogram(...)
}

attribute_scattergram <- function(...) {
  # Computes scatterplot summary statistics between two numeric vector fields.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$attribute_scattergram(...)
}

average_flowpath_slope <- function(...) {
  # Calculates average slope gradient of flowpaths passing through each DEM cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$average_flowpath_slope(...)
}

average_normal_vector_angular_deviation <- function(...) {
  # Calculates local mean angular deviation between original and smoothed surface normals.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$average_normal_vector_angular_deviation(...)
}

average_overlay <- function(...) {
  # Computes the per-cell average across a raster stack, ignoring NoData unless all inputs are NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$average_overlay(...)
}

average_upslope_flowpath_length <- function(...) {
  # Computes the average upslope flowpath length passing through each DEM cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$average_upslope_flowpath_length(...)
}

balance_contrast_enhancement <- function(...) {
  # Reduces colour bias in a packed RGB image using per-channel parabolic stretches.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$balance_contrast_enhancement(...)
}

basins <- function(...) {
  # Delineates all D8 drainage basins that drain to valid-data edges.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$basins(...)
}

bilateral_filter <- function(...) {
  # Performs an edge-preserving bilateral smoothing filter on a raster image.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$bilateral_filter(...)
}

block_maximum <- function(...) {
  # Rasterizes point features by assigning the maximum value observed within each output cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$block_maximum(...)
}

block_minimum <- function(...) {
  # Rasterizes point features by assigning the minimum value observed within each output cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$block_minimum(...)
}

bool_and <- function(...) {
  # Computes a logical AND of two rasters on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$bool_and(...)
}

bool_not <- function(...) {
  # Computes a logical NOT of each raster cell, outputting 1 for zero-valued cells and 0 otherwise.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$bool_not(...)
}

bool_or <- function(...) {
  # Computes a logical OR of two rasters on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$bool_or(...)
}

bool_xor <- function(...) {
  # Computes a logical XOR of two rasters on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$bool_xor(...)
}

boundary_shape_complexity <- function(...) {
  # Calculates raster patch boundary-shape complexity using a line-thinned skeleton branch metric.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$boundary_shape_complexity(...)
}

breach_depressions_least_cost <- function(...) {
  # Breaches depressions in a DEM using a constrained least-cost pathway search.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$breach_depressions_least_cost(...)
}

breach_single_cell_pits <- function(...) {
  # Breaches single-cell pits in a DEM by carving one-cell channels.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$breach_single_cell_pits(...)
}

buffer_raster <- function(...) {
  # Creates a binary buffer zone around non-zero, non-NoData raster cells within a specified distance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$buffer_raster(...)
}

buffer_vector <- function(...) {
  # Creates polygon buffers around point, line, and polygon vector geometries with configurable cap and join styles.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$buffer_vector(...)
}

burn_streams <- function(...) {
  # Burns a stream network into a DEM by decreasing stream-cell elevations.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$burn_streams(...)
}

burn_streams_at_roads <- function(...) {
  # Lowers stream elevations near stream-road crossings to breach road embankments in a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$burn_streams_at_roads(...)
}

canny_edge_detection <- function(...) {
  # Applies Canny multi-stage edge detection (Gaussian blur → Sobel gradient → non-maximum suppression → double threshold → hysteresis).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$canny_edge_detection(...)
}

casorati_curvature <- function(...) {
  # Calculates Casorati curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$casorati_curvature(...)
}

ceil <- function(...) {
  # Rounds each raster cell upward to the nearest integer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ceil(...)
}

centroid_raster <- function(...) {
  # Calculates the centroid cell for each positive-valued patch ID in a raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$centroid_raster(...)
}

centroid_vector <- function(...) {
  # Computes centroid points from vector features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$centroid_vector(...)
}

change_vector_analysis <- function(...) {
  # Performs change vector analysis on two-date multispectral datasets and returns magnitude and direction rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$change_vector_analysis(...)
}

circular_variance_of_aspect <- function(...) {
  # Calculates local circular variance of aspect within a moving neighbourhood.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$circular_variance_of_aspect(...)
}

classify_buildings_in_lidar <- function(...) {
  # Assigns classification 6 to LiDAR points falling inside building footprint polygons.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$classify_buildings_in_lidar(...)
}

classify_lidar <- function(...) {
  # Performs LiDAR classification into ground, building, and vegetation using neighborhood geometry and segmentation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$classify_lidar(...)
}

classify_overlap_points <- function(...) {
  # Flags or filters LiDAR points in grid cells containing multiple point source IDs.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$classify_overlap_points(...)
}

clean_vector <- function(...) {
  # Removes null and invalid vector geometries (e.g., undersized lines/polygons) while preserving valid features and attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$clean_vector(...)
}

clip <- function(...) {
  # Clips input polygons to overlay polygon boundaries using topology-based intersection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$clip(...)
}

clip_lidar_to_polygon <- function(...) {
  # Retains only LiDAR points that fall within polygon geometry.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$clip_lidar_to_polygon(...)
}

clip_raster_to_polygon <- function(...) {
  # Clips a raster to polygon extents; outside polygon cells are set to NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$clip_raster_to_polygon(...)
}

closing <- function(...) {
  # Performs a morphological closing operation using a rectangular structuring element.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$closing(...)
}

clump <- function(...) {
  # Groups contiguous equal-valued raster cells into unique patch identifiers.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$clump(...)
}

colourize_based_on_class <- function(...) {
  # Sets LiDAR point RGB values based on point classifications.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$colourize_based_on_class(...)
}

colourize_based_on_point_returns <- function(...) {
  # Sets LiDAR point RGB values based on return-type categories.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$colourize_based_on_point_returns(...)
}

compactness_ratio <- function(...) {
  # Computes compactness ratio (perimeter of equivalent circle / actual perimeter) for polygon features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$compactness_ratio(...)
}

concave_hull <- function(...) {
  # Creates concave hull polygons around all input feature coordinates.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$concave_hull(...)
}

conditional_evaluation <- function(...) {
  # Performs if-then-else conditional evaluation on raster cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$conditional_evaluation(...)
}

conservative_smoothing_filter <- function(...) {
  # Performs conservative smoothing by clipping impulse outliers to neighborhood bounds.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$conservative_smoothing_filter(...)
}

construct_vector_tin <- function(...) {
  # Constructs a triangular irregular network (TIN) from an input point set using Delaunay triangulation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$construct_vector_tin(...)
}

contours_from_points <- function(...) {
  # Creates contour polylines from point elevations using a Delaunay TIN.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$contours_from_points(...)
}

contours_from_raster <- function(...) {
  # Creates contour polylines from a raster surface model.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$contours_from_raster(...)
}

convergence_index <- function(...) {
  # Calculates the convergence/divergence index from local neighbour aspect alignment.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$convergence_index(...)
}

convert_nodata_to_zero <- function(...) {
  # Replaces raster nodata cells with 0 while leaving valid cells unchanged.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$convert_nodata_to_zero(...)
}

corner_detection <- function(...) {
  # Identifies corner patterns in binary rasters using hit-and-miss templates.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$corner_detection(...)
}

correct_vignetting <- function(...) {
  # Reduces brightness fall-off away from a principal point using a cosine lens model.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$correct_vignetting(...)
}

cos <- function(...) {
  # Computes the cosine of each raster cell value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$cos(...)
}

cosh <- function(...) {
  # Computes the hyperbolic cosine of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$cosh(...)
}

cost_allocation <- function(...) {
  # Assigns each cell to a source region using a backlink raster from cost distance analysis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$cost_allocation(...)
}

cost_distance <- function(...) {
  # Computes accumulated travel cost and backlink rasters from source and cost surfaces.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$cost_distance(...)
}

cost_pathway <- function(...) {
  # Traces least-cost pathways from destination cells using a backlink raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$cost_pathway(...)
}

count_if <- function(...) {
  # Counts the number of input rasters whose cell equals a comparison value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$count_if(...)
}

create_colour_composite <- function(...) {
  # Creates a packed RGB colour composite from red, green, blue, and optional opacity rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$create_colour_composite(...)
}

create_plane <- function(...) {
  # Creates a raster from a planar equation using a base raster geometry.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$create_plane(...)
}

crispness_index <- function(...) {
  # Calculates the crispness index for a membership probability raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$crispness_index(...)
}

cross_tabulation <- function(...) {
  # Performs cross-tabulation on two categorical rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$cross_tabulation(...)
}

csv_points_to_vector <- function(...) {
  # Imports point records from a CSV file into a point vector layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$csv_points_to_vector(...)
}

cumulative_distribution <- function(...) {
  # Converts raster values to cumulative distribution probabilities.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$cumulative_distribution(...)
}

curvedness <- function(...) {
  # Calculates the curvedness surface form descriptor from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$curvedness(...)
}

d8_flow_accum <- function(...) {
  # Calculates D8 flow accumulation from a DEM or D8 pointer raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$d8_flow_accum(...)
}

d8_mass_flux <- function(...) {
  # Performs a D8-based mass-flux accumulation using loading, efficiency, and absorption rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$d8_mass_flux(...)
}

d8_pointer <- function(...) {
  # Generates a D8 flow-direction pointer raster from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$d8_pointer(...)
}

dbscan <- function(...) {
  # Performs unsupervised DBSCAN density-based clustering on a stack of input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$dbscan(...)
}

decrement <- function(...) {
  # Subtracts 1 from each non-nodata raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$decrement(...)
}

delete_field <- function(...) {
  # Deletes one or more attribute fields from a vector layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$delete_field(...)
}

dem_void_filling <- function(...) {
  # Fills DEM voids using a secondary surface and interpolated elevation offsets for seamless fusion.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$dem_void_filling(...)
}

densify_features <- function(...) {
  # Adds vertices along line and polygon boundaries at a specified spacing.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$densify_features(...)
}

depth_in_sink <- function(...) {
  # Measures the depth each DEM cell lies below a depression-filled surface.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$depth_in_sink(...)
}

depth_to_water <- function(...) {
  # Computes cartographic depth-to-water using least-cost accumulation from stream/lake source features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$depth_to_water(...)
}

deviation_from_mean_elevation <- function(...) {
  # Calculates the local topographic z-score using local mean and standard deviation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$deviation_from_mean_elevation(...)
}

deviation_from_regional_direction <- function(...) {
  # Calculates polygon directional deviation from weighted regional mean orientation and appends DEV_DIR.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$deviation_from_regional_direction(...)
}

diff_of_gaussians_filter <- function(...) {
  # Performs Difference-of-Gaussians band-pass filtering.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$diff_of_gaussians_filter(...)
}

difference <- function(...) {
  # Removes overlay polygon areas from input polygons using topology-based difference.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$difference(...)
}

difference_curvature <- function(...) {
  # Calculates difference curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$difference_curvature(...)
}

difference_from_mean_elevation <- function(...) {
  # Calculates the difference between each elevation and the local mean elevation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$difference_from_mean_elevation(...)
}

dinf_flow_accum <- function(...) {
  # Calculates D-Infinity flow accumulation from a DEM or D-Infinity pointer raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$dinf_flow_accum(...)
}

dinf_mass_flux <- function(...) {
  # Performs a D-Infinity mass-flux accumulation using loading, efficiency, and absorption rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$dinf_mass_flux(...)
}

dinf_pointer <- function(...) {
  # Generates a D-Infinity flow-direction raster from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$dinf_pointer(...)
}

direct_decorrelation_stretch <- function(...) {
  # Improves packed RGB colour saturation by reducing the achromatic component and linearly stretching channels.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$direct_decorrelation_stretch(...)
}

directional_relief <- function(...) {
  # Calculates directional relief by ray-tracing elevation in a specified azimuth.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$directional_relief(...)
}

dissolve <- function(...) {
  # Removes shared polygon boundaries globally or by a dissolve attribute field.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$dissolve(...)
}

distance_to_outlet <- function(...) {
  # Calculates downstream distance to outlet for each stream cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$distance_to_outlet(...)
}

diversity_filter <- function(...) {
  # Computes moving-window diversity (count of unique values).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$diversity_filter(...)
}

divide <- function(...) {
  # Divides the first raster by the second on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$divide(...)
}

downslope_distance_to_stream <- function(...) {
  # Computes downslope distance from each DEM cell to nearest stream along flow paths.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$downslope_distance_to_stream(...)
}

downslope_flowpath_length <- function(...) {
  # Computes downslope flowpath length from each cell to an outlet in a D8 pointer raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$downslope_flowpath_length(...)
}

downslope_index <- function(...) {
  # Calculates Hjerdt et al. (2004) downslope index using D8 flow directions.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$downslope_index(...)
}

edge_contamination <- function(...) {
  # Identifies DEM cells whose upslope area extends beyond the DEM edge for common flow-routing schemes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$edge_contamination(...)
}

edge_density <- function(...) {
  # Calculates local density of breaks-in-slope using angular normal-vector differences.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$edge_density(...)
}

edge_preserving_mean_filter <- function(...) {
  # Performs thresholded edge-preserving mean filtering.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$edge_preserving_mean_filter(...)
}

edge_proportion <- function(...) {
  # Calculates the proportion of each patch's cells that are edge cells and maps it back to patch cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$edge_proportion(...)
}

elev_above_pit <- function(...) {
  # Calculates elevation above the nearest downslope pit cell (or edge sink).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elev_above_pit(...)
}

elev_above_pit_dist <- function(...) {
  # Compatibility alias for elev_above_pit.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elev_above_pit_dist(...)
}

elev_relative_to_min_max <- function(...) {
  # Expresses each elevation as a percentage (0–100) of the raster's elevation range.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elev_relative_to_min_max(...)
}

elev_relative_to_watershed_min_max <- function(...) {
  # Calculates a DEM cell's relative elevation position within each watershed as a percentage.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elev_relative_to_watershed_min_max(...)
}

elevation_above_stream <- function(...) {
  # Computes elevation above nearest stream measured along downslope flow paths.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elevation_above_stream(...)
}

elevation_above_stream_euclidean <- function(...) {
  # Computes elevation above nearest stream using straight-line (Euclidean) proximity.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elevation_above_stream_euclidean(...)
}

elevation_percentile <- function(...) {
  # Calculates the local percentile rank of each cell elevation within a neighbourhood window.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elevation_percentile(...)
}

eliminate_coincident_points <- function(...) {
  # Removes coincident or near-coincident points within a tolerance distance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$eliminate_coincident_points(...)
}

elongation_ratio <- function(...) {
  # Computes elongation ratio (short axis / long axis of bounding rectangle) for polygon features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$elongation_ratio(...)
}

embankment_mapping <- function(...) {
  # Maps transportation embankments from a DEM and road network, with optional embankment-surface removal via interpolation. Authored by John Lindsay and Nigel VanNieuwenhuizen.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$embankment_mapping(...)
}

emboss_filter <- function(...) {
  # Performs directional emboss filtering.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$emboss_filter(...)
}

equal_to <- function(...) {
  # Tests whether two rasters are equal on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$equal_to(...)
}

erase <- function(...) {
  # Erases overlay polygon areas from input polygons and preserves input attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$erase(...)
}

erase_polygon_from_lidar <- function(...) {
  # Removes LiDAR points that fall within polygon geometry.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$erase_polygon_from_lidar(...)
}

erase_polygon_from_raster <- function(...) {
  # Sets raster cells inside polygons to NoData while preserving cells in polygon holes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$erase_polygon_from_raster(...)
}

euclidean_allocation <- function(...) {
  # Assigns each valid cell the value of its nearest non-zero target cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$euclidean_allocation(...)
}

euclidean_distance <- function(...) {
  # Computes Euclidean distance to nearest non-zero target cell in a raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$euclidean_distance(...)
}

evaluate_training_sites <- function(...) {
  # Evaluates class separability in multi-band training polygons and writes an HTML report with per-band distribution statistics.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$evaluate_training_sites(...)
}

exp <- function(...) {
  # Computes e raised to the power of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$exp(...)
}

exp2 <- function(...) {
  # Computes 2 raised to the power of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$exp2(...)
}

export_table_to_csv <- function(...) {
  # Exports a vector attribute table to a CSV file.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$export_table_to_csv(...)
}

exposure_towards_wind_flux <- function(...) {
  # Calculates terrain exposure relative to dominant wind direction and upwind horizon shielding.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$exposure_towards_wind_flux(...)
}

extend_vector_lines <- function(...) {
  # Extends polyline endpoints by a specified distance at the start, end, or both.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$extend_vector_lines(...)
}

extract_by_attribute <- function(...) {
  # Extracts vector features that satisfy an attribute expression.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$extract_by_attribute(...)
}

extract_nodes <- function(...) {
  # Converts polyline and polygon vertices into point features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$extract_nodes(...)
}

extract_raster_values_at_points <- function(...) {
  # Samples one or more rasters at point locations and writes the values to point attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$extract_raster_values_at_points(...)
}

extract_streams <- function(...) {
  # Extracts streams based on flow accumulation threshold.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$extract_streams(...)
}

extract_valleys <- function(...) {
  # Extracts valleys from DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$extract_valleys(...)
}

farthest_channel_head <- function(...) {
  # Calculates distance to most distant channel head.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$farthest_channel_head(...)
}

fast_almost_gaussian_filter <- function(...) {
  # Performs a fast approximation to Gaussian smoothing.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fast_almost_gaussian_filter(...)
}

fd8_flow_accum <- function(...) {
  # Calculates FD8 flow accumulation from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fd8_flow_accum(...)
}

fd8_pointer <- function(...) {
  # Generates an FD8 multiple-flow-direction pointer raster from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fd8_pointer(...)
}

feature_preserving_smoothing <- function(...) {
  # Smooths DEM roughness while preserving breaks-in-slope using normal-vector filtering.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$feature_preserving_smoothing(...)
}

fetch_analysis <- function(...) {
  # Computes upwind distance to the first topographic obstacle along a specified azimuth.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fetch_analysis(...)
}

field_calculator <- function(...) {
  # Calculates a field value from an expression using feature attributes and geometry variables.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$field_calculator(...)
}

fill_burn <- function(...) {
  # Hydro-enforces a DEM by burning streams and then filling depressions.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fill_burn(...)
}

fill_depressions <- function(...) {
  # Fills depressions in a DEM using a priority-flood strategy with optional flat resolution.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fill_depressions(...)
}

fill_depressions_planchon_and_darboux <- function(...) {
  # Fills depressions in a DEM with a Planchon-and-Darboux-compatible interface.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fill_depressions_planchon_and_darboux(...)
}

fill_depressions_wang_and_liu <- function(...) {
  # Fills depressions in a DEM with a Wang-and-Liu-compatible interface.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fill_depressions_wang_and_liu(...)
}

fill_missing_data <- function(...) {
  # Fills NoData gaps using inverse-distance weighting from valid gap-edge cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fill_missing_data(...)
}

fill_pits <- function(...) {
  # Fills single-cell pits in a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fill_pits(...)
}

filter_lidar <- function(...) {
  # Filters LiDAR points using a boolean expression over point attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_lidar(...)
}

filter_lidar_by_percentile <- function(...) {
  # Selects one representative point per grid block based on elevation percentile.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_lidar_by_percentile(...)
}

filter_lidar_by_reference_surface <- function(...) {
  # Extracts or classifies points based on z relation to a reference raster surface.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_lidar_by_reference_surface(...)
}

filter_lidar_classes <- function(...) {
  # Removes points that match excluded classification values.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_lidar_classes(...)
}

filter_lidar_noise <- function(...) {
  # Removes low (class 7) and high (class 18) noise-classified points from a LiDAR file.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_lidar_noise(...)
}

filter_lidar_scan_angles <- function(...) {
  # Removes LiDAR points whose absolute scan angle exceeds a threshold.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_lidar_scan_angles(...)
}

filter_raster_features_by_area <- function(...) {
  # Removes integer-labelled raster features smaller than a cell-count threshold.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_raster_features_by_area(...)
}

filter_vector_features_by_area <- function(...) {
  # Filters polygon features below a minimum area threshold.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$filter_vector_features_by_area(...)
}

find_flightline_edge_points <- function(...) {
  # Extracts only points flagged as edge-of-flightline.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$find_flightline_edge_points(...)
}

find_lowest_or_highest_points <- function(...) {
  # Locates lowest and/or highest raster cells and outputs their locations as points.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$find_lowest_or_highest_points(...)
}

find_main_stem <- function(...) {
  # Identifies main stem of stream network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$find_main_stem(...)
}

find_noflow_cells <- function(...) {
  # Finds DEM cells that have no lower D8 neighbour.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$find_noflow_cells(...)
}

find_parallel_flow <- function(...) {
  # Identifies stream cells that possess parallel D8 flow directions.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$find_parallel_flow(...)
}

find_patch_edge_cells <- function(...) {
  # Identifies edge cells for each positive raster patch ID; non-edge patch cells are set to zero.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$find_patch_edge_cells(...)
}

find_ridges <- function(...) {
  # Identifies potential ridge and peak cells in a DEM, with optional line thinning.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$find_ridges(...)
}

fix_dangling_arcs <- function(...) {
  # Fixes undershot and overshot dangling arcs in a line network by snapping line endpoints within a threshold distance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fix_dangling_arcs(...)
}

flatten_lakes <- function(...) {
  # Flattens lake elevations using minimum perimeter elevation for each polygon.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$flatten_lakes(...)
}

flightline_overlap <- function(...) {
  # Counts distinct point-source IDs per raster cell to identify overlapping flightlines.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$flightline_overlap(...)
}

flip_image <- function(...) {
  # Flips an image vertically, horizontally, or both.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$flip_image(...)
}

flood_order <- function(...) {
  # Outputs the sequential priority-flood order for each DEM cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$flood_order(...)
}

floor <- function(...) {
  # Rounds each raster cell downward to the nearest integer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$floor(...)
}

flow_accum_full_workflow <- function(...) {
  # Runs a full non-divergent flow-accumulation workflow and returns breached DEM, flow-direction pointer, and accumulation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$flow_accum_full_workflow(...)
}

flow_length_diff <- function(...) {
  # Computes local maximum absolute differences in downslope path length from a D8 pointer raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$flow_length_diff(...)
}

frangi_filter <- function(...) {
  # Performs multiscale Frangi vesselness enhancement.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$frangi_filter(...)
}

frost_filter <- function(...) {
  # Performs adaptive Frost speckle filtering for radar imagery.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$frost_filter(...)
}

fuzzy_knn_classification <- function(...) {
  # Performs fuzzy k-nearest-neighbor classification and outputs class membership confidence.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$fuzzy_knn_classification(...)
}

gabor_filter_bank <- function(...) {
  # Performs multi-orientation Gabor response filtering.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$gabor_filter_bank(...)
}

gamma_correction <- function(...) {
  # Applies gamma intensity correction to grayscale or RGB imagery.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$gamma_correction(...)
}

gamma_map_filter <- function(...) {
  # Performs Gamma-MAP speckle filtering for radar imagery.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$gamma_map_filter(...)
}

gaussian_contrast_stretch <- function(...) {
  # Stretches contrast by matching to a Gaussian reference distribution.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$gaussian_contrast_stretch(...)
}

gaussian_curvature <- function(...) {
  # Calculates Gaussian curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$gaussian_curvature(...)
}

gaussian_filter <- function(...) {
  # Performs Gaussian smoothing on a raster image.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$gaussian_filter(...)
}

generalize_classified_raster <- function(...) {
  # Generalizes small class patches by merging them into neighboring larger classes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$generalize_classified_raster(...)
}

generalize_with_similarity <- function(...) {
  # Generalizes small patches in a classified raster by merging them into the most spectrally similar neighboring patch.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$generalize_with_similarity(...)
}

generating_function <- function(...) {
  # Calculates generating function from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$generating_function(...)
}

geomorphons <- function(...) {
  # Classifies landforms using 8-direction line-of-sight ternary patterns based on zenith-nadir angle differences, or 10 common geomorphon forms. Authored by Dan Newman and John Lindsay.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$geomorphons(...)
}

greater_than <- function(...) {
  # Tests whether the first raster is greater than the second on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$greater_than(...)
}

guided_filter <- function(...) {
  # Performs edge-preserving guided filtering using local linear models.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$guided_filter(...)
}

hack_stream_order <- function(...) {
  # Assigns Hack stream order to stream cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hack_stream_order(...)
}

heat_map <- function(...) {
  # Generates a kernel-density heat map raster from point occurrences.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$heat_map(...)
}

height_above_ground <- function(...) {
  # Converts LiDAR elevations to heights above the nearest ground-classified point.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$height_above_ground(...)
}

hexagonal_grid_from_raster_base <- function(...) {
  # Creates a hexagonal polygon grid covering a raster extent.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hexagonal_grid_from_raster_base(...)
}

hexagonal_grid_from_vector_base <- function(...) {
  # Creates a hexagonal polygon grid covering a vector-layer bounding extent.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hexagonal_grid_from_vector_base(...)
}

high_pass_bilateral_filter <- function(...) {
  # Computes a high-pass residual by subtracting bilateral smoothing from the input raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$high_pass_bilateral_filter(...)
}

high_pass_filter <- function(...) {
  # Performs high-pass filtering using neighborhood mean subtraction.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$high_pass_filter(...)
}

high_pass_median_filter <- function(...) {
  # Performs high-pass filtering by subtracting local median from center values.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$high_pass_median_filter(...)
}

highest_position <- function(...) {
  # Returns the zero-based raster-stack index containing the highest value at each cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$highest_position(...)
}

hillshade <- function(...) {
  # Produces shaded-relief from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hillshade(...)
}

hillslopes <- function(...) {
  # Identifies hillslope regions draining to each stream link, separating left- and right-bank areas.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hillslopes(...)
}

histogram_equalization <- function(...) {
  # Applies histogram equalization to improve image contrast.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$histogram_equalization(...)
}

histogram_matching <- function(...) {
  # Matches an image histogram to a supplied reference histogram.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$histogram_matching(...)
}

histogram_matching_two_images <- function(...) {
  # Matches an input image histogram to a reference image histogram.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$histogram_matching_two_images(...)
}

hole_proportion <- function(...) {
  # Calculates polygon hole area divided by hull area and appends HOLE_PROP.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hole_proportion(...)
}

horizon_angle <- function(...) {
  # Calculates horizon angle (maximum slope) along a specified azimuth direction.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$horizon_angle(...)
}

horizontal_excess_curvature <- function(...) {
  # Calculates horizontal excess curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$horizontal_excess_curvature(...)
}

horton_ratios <- function(...) {
  # Calculates Horton bifurcation, length, drainage-area, and slope ratios.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$horton_ratios(...)
}

horton_stream_order <- function(...) {
  # Assigns Horton stream order to stream cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$horton_stream_order(...)
}

hydrologic_connectivity <- function(...) {
  # Computes DUL and UDSA connectivity indices from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hydrologic_connectivity(...)
}

hypsometric_analysis <- function(...) {
  # Creates a hypsometric (area-elevation) curve HTML report for one or more DEMs.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hypsometric_analysis(...)
}

hypsometrically_tinted_hillshade <- function(...) {
  # Creates a Swiss-style terrain rendering by blending multi-azimuth hillshade with hypsometric tinting and optional atmospheric haze.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$hypsometrically_tinted_hillshade(...)
}

idw_interpolation <- function(...) {
  # Interpolates a raster from point samples using inverse-distance weighting.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$idw_interpolation(...)
}

ihs_to_rgb <- function(...) {
  # Converts intensity, hue, and saturation band rasters back to red, green, and blue channels (0–255).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ihs_to_rgb(...)
}

image_autocorrelation <- function(...) {
  # Computes Moran's I for one or more raster images.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$image_autocorrelation(...)
}

image_correlation <- function(...) {
  # Computes Pearson correlation matrix for two or more raster images.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$image_correlation(...)
}

image_correlation_neighbourhood_analysis <- function(...) {
  # Performs moving-window correlation analysis between two rasters and returns correlation and p-value rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$image_correlation_neighbourhood_analysis(...)
}

image_regression <- function(...) {
  # Performs bivariate linear regression between two rasters and outputs a residual raster and report.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$image_regression(...)
}

image_segmentation <- function(...) {
  # Segments multi-band raster stacks into contiguous homogeneous regions using seeded region growing.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$image_segmentation(...)
}

image_slider <- function(...) {
  # Creates an interactive HTML image slider from two raster images.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$image_slider(...)
}

image_stack_profile <- function(...) {
  # Extracts per-point profiles across an ordered raster stack and optionally writes an HTML report.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$image_stack_profile(...)
}

impoundment_size_index <- function(...) {
  # Computes mean/max depth, volume, area, and dam-height impoundment metrics.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$impoundment_size_index(...)
}

improved_ground_point_filter <- function(...) {
  # Multi-stage ground point filtering pipeline.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$improved_ground_point_filter(...)
}

increment <- function(...) {
  # Adds 1 to each non-nodata raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$increment(...)
}

individual_tree_detection <- function(...) {
  # Identifies tree top points in a LiDAR cloud using local maxima detection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$individual_tree_detection(...)
}

individual_tree_segmentation <- function(...) {
  # Segments vegetation LiDAR points into individual tree clusters using a mean-shift mode-seeking workflow.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$individual_tree_segmentation(...)
}

inplace_add <- function(...) {
  # Performs an in-place addition operation (input1 += input2).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$inplace_add(...)
}

inplace_divide <- function(...) {
  # Performs an in-place division operation (input1 /= input2).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$inplace_divide(...)
}

inplace_multiply <- function(...) {
  # Performs an in-place multiplication operation (input1 *= input2).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$inplace_multiply(...)
}

inplace_subtract <- function(...) {
  # Performs an in-place subtraction operation (input1 -= input2).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$inplace_subtract(...)
}

insert_dams <- function(...) {
  # Adds local dam embankments at specified points using profile-based crest selection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$insert_dams(...)
}

integer_division <- function(...) {
  # Divides two rasters and truncates each result toward zero.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$integer_division(...)
}

integral_image_transform <- function(...) {
  # Computes a summed-area (integral image) transform for each band.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$integral_image_transform(...)
}

intersect <- function(...) {
  # Intersects input and overlay polygons using topology-based overlay and tracks source feature IDs.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$intersect(...)
}

inverse_pca <- function(...) {
  # Reconstructs original band images from PCA component rasters using stored eigenvectors.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$inverse_pca(...)
}

is_nodata <- function(...) {
  # Outputs 1 for nodata cells and 0 for all valid cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$is_nodata(...)
}

isobasins <- function(...) {
  # Divides a landscape into approximately equal-sized watersheds (isobasins) based on a target area threshold.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$isobasins(...)
}

jenson_snap_pour_points <- function(...) {
  # Snaps each pour point to the nearest stream cell within a search distance, preserving all input attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$jenson_snap_pour_points(...)
}

join_tables <- function(...) {
  # Joins attributes from a foreign vector table to a primary vector table using key fields.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$join_tables(...)
}

k_means_clustering <- function(...) {
  # Performs k-means clustering on a multi-band raster stack and outputs a categorical class raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$k_means_clustering(...)
}

k_nearest_mean_filter <- function(...) {
  # Performs edge-preserving k-nearest neighbor mean smoothing.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$k_nearest_mean_filter(...)
}

k_shortest_paths_network <- function(...) {
  # Finds the k shortest simple paths between start and end coordinates over a line network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$k_shortest_paths_network(...)
}

kappa_index <- function(...) {
  # Computes Cohen's kappa and agreement metrics between two categorical rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$kappa_index(...)
}

knn_classification <- function(...) {
  # Performs supervised k-nearest-neighbor classification on multi-band input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$knn_classification(...)
}

knn_regression <- function(...) {
  # Performs supervised k-nearest-neighbor regression on multi-band input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$knn_regression(...)
}

ks_normality_test <- function(...) {
  # Evaluates whether raster values are drawn from a normal distribution.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ks_normality_test(...)
}

kuan_filter <- function(...) {
  # Performs Kuan speckle filtering for radar imagery.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$kuan_filter(...)
}

kuwahara_filter <- function(...) {
  # Performs edge-preserving Kuwahara filtering using minimum-variance subwindows.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$kuwahara_filter(...)
}

laplacian_filter <- function(...) {
  # Performs Laplacian edge/sharpen filtering.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$laplacian_filter(...)
}

laplacian_of_gaussians_filter <- function(...) {
  # Performs Laplacian-of-Gaussians edge enhancement.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$laplacian_of_gaussians_filter(...)
}

las_to_ascii <- function(...) {
  # Converts LiDAR points to CSV ASCII text.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$las_to_ascii(...)
}

las_to_shapefile <- function(...) {
  # Converts LAS/LAZ point clouds into vector point shapefiles.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$las_to_shapefile(...)
}

layer_footprint_raster <- function(...) {
  # Creates a polygon footprint representing the full extent of an input raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$layer_footprint_raster(...)
}

layer_footprint_vector <- function(...) {
  # Creates a polygon footprint representing the full bounding extent of an input vector layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$layer_footprint_vector(...)
}

lee_filter <- function(...) {
  # Performs Lee sigma filtering using in-range neighborhood averaging.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lee_filter(...)
}

length_of_upstream_channels <- function(...) {
  # Calculates total upstream channel length.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$length_of_upstream_channels(...)
}

less_than <- function(...) {
  # Tests whether the first raster is less than the second on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$less_than(...)
}

lidar_block_maximum <- function(...) {
  # Creates a raster by assigning each cell the maximum value of included LiDAR points.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_block_maximum(...)
}

lidar_block_minimum <- function(...) {
  # Creates a raster by assigning each cell the minimum value of included LiDAR points.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_block_minimum(...)
}

lidar_classify_subset <- function(...) {
  # Classifies points in a base LiDAR cloud that spatially match points in a subset cloud.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_classify_subset(...)
}

lidar_colourize <- function(...) {
  # Assigns LiDAR point RGB values from an overlapping raster image.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_colourize(...)
}

lidar_construct_vector_tin <- function(...) {
  # Creates a vector TIN (triangular mesh) from LiDAR points.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_construct_vector_tin(...)
}

lidar_contour <- function(...) {
  # Creates contour vector lines from a LiDAR point cloud using TIN contouring.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_contour(...)
}

lidar_digital_surface_model <- function(...) {
  # Builds a DSM from top-surface LiDAR points and TIN interpolation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_digital_surface_model(...)
}

lidar_eigenvalue_features <- function(...) {
  # Computes local PCA-based LiDAR neighbourhood features and writes a .eigen binary with JSON sidecar.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_eigenvalue_features(...)
}

lidar_elevation_slice <- function(...) {
  # Extracts or reclassifies LiDAR points within a specified elevation range.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_elevation_slice(...)
}

lidar_ground_point_filter <- function(...) {
  # Slope-based filtering/classification of off-terrain points in LiDAR data.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_ground_point_filter(...)
}

lidar_hex_bin <- function(...) {
  # Bins LiDAR points into a hexagonal grid and outputs per-cell summary attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_hex_bin(...)
}

lidar_hillshade <- function(...) {
  # Creates a hillshade raster from LiDAR elevations using local block maxima as surface input.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_hillshade(...)
}

lidar_histogram <- function(...) {
  # Builds a simple histogram report for a selected LiDAR attribute.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_histogram(...)
}

lidar_idw_interpolation <- function(...) {
  # Interpolates a raster from LiDAR points using inverse-distance weighting.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_idw_interpolation(...)
}

lidar_info <- function(...) {
  # Generates a textual or HTML summary report for a LiDAR file.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_info(...)
}

lidar_join <- function(...) {
  # Merges multiple LiDAR files into a single output point cloud.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_join(...)
}

lidar_kappa <- function(...) {
  # Computes a kappa agreement report between two classified LiDAR clouds and writes a class-agreement raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_kappa(...)
}

lidar_nearest_neighbour_gridding <- function(...) {
  # Interpolates a raster from LiDAR points using nearest-neighbour assignment.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_nearest_neighbour_gridding(...)
}

lidar_point_density <- function(...) {
  # Computes point density from LiDAR samples within a moving-radius neighbourhood.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_point_density(...)
}

lidar_point_return_analysis <- function(...) {
  # Runs return-sequence QC analysis and writes a text report; optionally writes a classified QC LiDAR output.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_point_return_analysis(...)
}

lidar_point_stats <- function(...) {
  # Creates one or more raster grids summarizing LiDAR point distributions.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_point_stats(...)
}

lidar_radial_basis_function_interpolation <- function(...) {
  # Interpolates a raster from LiDAR points using local radial-basis similarity weighting.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_radial_basis_function_interpolation(...)
}

lidar_ransac_planes <- function(...) {
  # Identifies locally planar LiDAR points using neighbourhood RANSAC plane fitting.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_ransac_planes(...)
}

lidar_remove_outliers <- function(...) {
  # Filters or classifies outlier points based on local elevation residuals.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_remove_outliers(...)
}

lidar_rooftop_analysis <- function(...) {
  # Identifies planar rooftop segments within building footprints and outputs segment polygons with roof attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_rooftop_analysis(...)
}

lidar_segmentation <- function(...) {
  # Segments a LiDAR cloud into connected components and assigns segment colours.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_segmentation(...)
}

lidar_segmentation_based_filter <- function(...) {
  # Ground-point filtering based on neighbourhood-connected low-relief segments.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_segmentation_based_filter(...)
}

lidar_shift <- function(...) {
  # Shifts LiDAR point coordinates by x/y/z offsets.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_shift(...)
}

lidar_sibson_interpolation <- function(...) {
  # Interpolates a raster from LiDAR points using true Sibson natural-neighbour interpolation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_sibson_interpolation(...)
}

lidar_thin <- function(...) {
  # Thins a LiDAR point cloud by retaining at most one point per grid cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_thin(...)
}

lidar_thin_high_density <- function(...) {
  # Thins points in locally high-density areas while preserving lower-density regions.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_thin_high_density(...)
}

lidar_tile <- function(...) {
  # Splits an input LiDAR file into a regular tile grid and writes one output per populated tile.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_tile(...)
}

lidar_tile_footprint <- function(...) {
  # Creates polygon footprints (bounding boxes or convex hulls) for LiDAR tiles.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_tile_footprint(...)
}

lidar_tin_gridding <- function(...) {
  # Interpolates a raster from LiDAR points using Delaunay triangulation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_tin_gridding(...)
}

lidar_tophat_transform <- function(...) {
  # Applies a white top-hat transform to LiDAR elevations to approximate height above local ground.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lidar_tophat_transform(...)
}

line_detection_filter <- function(...) {
  # Performs directional line detection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$line_detection_filter(...)
}

line_intersections <- function(...) {
  # Finds line intersection points between input and overlay layers and appends parent IDs with merged attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$line_intersections(...)
}

line_polygon_clip <- function(...) {
  # Clips line features to polygon interiors and outputs clipped line segments.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$line_polygon_clip(...)
}

line_thinning <- function(...) {
  # Reduces connected binary raster features to one-cell-wide skeleton lines.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$line_thinning(...)
}

linearity_index <- function(...) {
  # Computes linearity index (straight-line distance / actual length) for line and polygon features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$linearity_index(...)
}

lines_to_polygons <- function(...) {
  # Converts polyline features into polygon features, treating the first part as the exterior ring and later parts as holes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lines_to_polygons(...)
}

list_unique_values <- function(...) {
  # Lists unique values and frequencies in a vector attribute field.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$list_unique_values(...)
}

list_unique_values_raster <- function(...) {
  # Lists unique valid values in a raster (capped to protect memory).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$list_unique_values_raster(...)
}

ln <- function(...) {
  # Computes the natural logarithm of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ln(...)
}

locate_points_along_routes <- function(...) {
  # Locates point features along route lines and writes route-measure attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$locate_points_along_routes(...)
}

log10 <- function(...) {
  # Computes the base-10 logarithm of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$log10(...)
}

log2 <- function(...) {
  # Computes the base-2 logarithm of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$log2(...)
}

logistic_regression <- function(...) {
  # Performs supervised logistic regression classification on multi-band input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$logistic_regression(...)
}

long_profile <- function(...) {
  # Creates longitudinal stream profile.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$long_profile(...)
}

long_profile_from_points <- function(...) {
  # Creates long profile from vector points.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$long_profile_from_points(...)
}

longest_flowpath <- function(...) {
  # Delineates longest flowpath lines for each basin in a basin raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$longest_flowpath(...)
}

lowest_position <- function(...) {
  # Returns the zero-based raster-stack index containing the lowest value at each cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$lowest_position(...)
}

majority_filter <- function(...) {
  # Computes moving-window mode (majority class/value).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$majority_filter(...)
}

map_features <- function(...) {
  # Maps discrete elevated terrain features from a raster using descending-priority region growth.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$map_features(...)
}

map_matching_v1 <- function(...) {
  # Snaps trajectory points onto a line network and reconstructs an inferred route with diagnostics.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$map_matching_v1(...)
}

map_off_terrain_objects <- function(...) {
  # Maps off-terrain object segments in DSMs using slope-constrained region growing and optional minimum feature-size filtering.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$map_off_terrain_objects(...)
}

max <- function(...) {
  # Performs a MAX operation on two rasters or a raster and a constant value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max(...)
}

max_absolute_overlay <- function(...) {
  # Computes the per-cell maximum absolute value across a raster stack, propagating NoData if any input cell is NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_absolute_overlay(...)
}

max_anisotropy_dev <- function(...) {
  # Calculates maximum anisotropy in elevation deviation over a range of neighbourhood scales. Written by Dan Newman.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_anisotropy_dev(...)
}

max_anisotropy_dev_signature <- function(...) {
  # Calculates multiscale anisotropy signatures for input point sites and writes an HTML report. Written by Dan Newman.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_anisotropy_dev_signature(...)
}

max_branch_length <- function(...) {
  # Calculates maximum branch length between neighbouring D8 flowpaths, useful for highlighting divides.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_branch_length(...)
}

max_difference_from_mean <- function(...) {
  # Calculates maximum absolute difference-from-mean over a range of neighbourhood scales.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_difference_from_mean(...)
}

max_downslope_elev_change <- function(...) {
  # Calculates the maximum elevation drop to lower neighbouring cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_downslope_elev_change(...)
}

max_elev_dev_signature <- function(...) {
  # Calculates multiscale elevation-deviation signatures for input point sites and writes an HTML report.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_elev_dev_signature(...)
}

max_elevation_deviation <- function(...) {
  # Calculates maximum standardized elevation deviation (DEVmax) over a range of neighbourhood scales.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_elevation_deviation(...)
}

max_overlay <- function(...) {
  # Computes the per-cell maximum across a raster stack, propagating NoData if any input cell is NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_overlay(...)
}

max_upslope_elev_change <- function(...) {
  # Calculates the maximum elevation gain to higher neighbouring cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_upslope_elev_change(...)
}

max_upslope_flowpath_length <- function(...) {
  # Computes the maximum upslope flowpath length passing through each DEM cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_upslope_flowpath_length(...)
}

max_upslope_value <- function(...) {
  # Propagates maximum upslope value along D8 flowpaths over a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$max_upslope_value(...)
}

maximal_curvature <- function(...) {
  # Calculates maximal (maximum principal) curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$maximal_curvature(...)
}

maximum_filter <- function(...) {
  # Computes a moving-window maximum for each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$maximum_filter(...)
}

mdinf_flow_accum <- function(...) {
  # Calculates MD-Infinity triangular multiple-flow-direction accumulation from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$mdinf_flow_accum(...)
}

mean_curvature <- function(...) {
  # Calculates mean curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$mean_curvature(...)
}

mean_filter <- function(...) {
  # Computes a moving-window mean for each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$mean_filter(...)
}

median_filter <- function(...) {
  # Computes moving-window median values.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$median_filter(...)
}

medoid <- function(...) {
  # Calculates medoid points from vector geometries.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$medoid(...)
}

merge_line_segments <- function(...) {
  # Merges connected line segments that meet at non-branching endpoints.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$merge_line_segments(...)
}

merge_table_with_csv <- function(...) {
  # Merges attributes from a CSV table into a vector attribute table by key fields.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$merge_table_with_csv(...)
}

merge_vectors <- function(...) {
  # Combines two or more input vectors of the same geometry type into a single output vector.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$merge_vectors(...)
}

min <- function(...) {
  # Performs a MIN operation on two rasters or a raster and a constant value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$min(...)
}

min_absolute_overlay <- function(...) {
  # Computes the per-cell minimum absolute value across a raster stack, propagating NoData if any input cell is NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$min_absolute_overlay(...)
}

min_dist_classification <- function(...) {
  # Performs a supervised minimum-distance classification on multi-spectral rasters using polygon training data.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$min_dist_classification(...)
}

min_downslope_elev_change <- function(...) {
  # Calculates the minimum non-negative elevation drop to neighbouring cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$min_downslope_elev_change(...)
}

min_max_contrast_stretch <- function(...) {
  # Linearly stretches values between user-specified minimum and maximum.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$min_max_contrast_stretch(...)
}

min_overlay <- function(...) {
  # Computes the per-cell minimum across a raster stack, propagating NoData if any input cell is NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$min_overlay(...)
}

minimal_curvature <- function(...) {
  # Calculates minimal (minimum principal) curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$minimal_curvature(...)
}

minimal_dispersion_flow_algorithm <- function(...) {
  # Generates MDFA flow-direction and flow-accumulation rasters from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$minimal_dispersion_flow_algorithm(...)
}

minimum_bounding_box <- function(...) {
  # Calculates oriented minimum bounding boxes around individual features or the entire layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$minimum_bounding_box(...)
}

minimum_bounding_circle <- function(...) {
  # Calculates minimum enclosing circles around individual features or the entire layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$minimum_bounding_circle(...)
}

minimum_bounding_envelope <- function(...) {
  # Calculates axis-aligned minimum bounding envelopes around individual features or the entire layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$minimum_bounding_envelope(...)
}

minimum_convex_hull <- function(...) {
  # Creates convex hull polygons around individual features or the full input layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$minimum_convex_hull(...)
}

minimum_filter <- function(...) {
  # Computes a moving-window minimum for each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$minimum_filter(...)
}

modified_k_means_clustering <- function(...) {
  # Performs modified k-means clustering with centroid merging based on a user-defined merge distance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$modified_k_means_clustering(...)
}

modified_shepard_interpolation <- function(...) {
  # Interpolates a raster from point samples using locally weighted modified-Shepard blending.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$modified_shepard_interpolation(...)
}

modify_lidar <- function(...) {
  # Applies assignment expressions to modify LiDAR point attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$modify_lidar(...)
}

modify_nodata_value <- function(...) {
  # Changes the raster nodata value and rewrites existing nodata cells to the new value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$modify_nodata_value(...)
}

modulo <- function(...) {
  # Computes the remainder of dividing the first raster by the second on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$modulo(...)
}

mosaic <- function(...) {
  # Mosaics two or more rasters into a new output raster using nearest-neighbour, bilinear, or cubic resampling.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$mosaic(...)
}

mosaic_with_feathering <- function(...) {
  # Mosaics two rasters and feather-blends overlapping cells using edge-distance weights.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$mosaic_with_feathering(...)
}

multidirectional_hillshade <- function(...) {
  # Produces weighted multi-azimuth shaded-relief.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multidirectional_hillshade(...)
}

multimodal_od_cost_matrix <- function(...) {
  # Computes batched multimodal OD costs and mode summaries between origin and destination point sets.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multimodal_od_cost_matrix(...)
}

multimodal_routes_from_od <- function(...) {
  # Builds route geometries for multimodal origin-destination point pairs with per-route mode summaries.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multimodal_routes_from_od(...)
}

multimodal_shortest_path <- function(...) {
  # Finds a mode-aware shortest path over a line network with configurable transfer penalties.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multimodal_shortest_path(...)
}

multipart_to_singlepart <- function(...) {
  # Converts a vector containing multi-part features into one with only single-part features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multipart_to_singlepart(...)
}

multiply <- function(...) {
  # Multiplies two rasters on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiply(...)
}

multiply_overlay <- function(...) {
  # Computes the per-cell product across a raster stack, propagating NoData if any input cell is NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiply_overlay(...)
}

multiscale_curvatures <- function(...) {
  # Calculates multiscale curvatures and curvature-based indices from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_curvatures(...)
}

multiscale_elevated_index <- function(...) {
  # Calculates multiscale elevated-index (MsEI) and key-scale rasters using Gaussian scale-space residuals.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_elevated_index(...)
}

multiscale_elevation_percentile <- function(...) {
  # Calculates the most extreme local elevation percentile across a range of neighbourhood scales.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_elevation_percentile(...)
}

multiscale_low_lying_index <- function(...) {
  # Calculates multiscale low-lying-index (MsLLI) and key-scale rasters using Gaussian scale-space residuals.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_low_lying_index(...)
}

multiscale_roughness <- function(...) {
  # Calculates surface roughness over a range of neighbourhood scales.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_roughness(...)
}

multiscale_roughness_signature <- function(...) {
  # Calculates multiscale roughness signatures for input point sites and writes an HTML report.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_roughness_signature(...)
}

multiscale_std_dev_normals <- function(...) {
  # Calculates maximum spherical standard deviation of surface normals over a nonlinearly sampled range of scales.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_std_dev_normals(...)
}

multiscale_std_dev_normals_signature <- function(...) {
  # Calculates spherical-standard-deviation scale signatures for input point sites and writes an HTML report.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_std_dev_normals_signature(...)
}

multiscale_topographic_position_image <- function(...) {
  # Creates a packed RGB multiscale topographic-position image from local, meso, and broad DEVmax rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$multiscale_topographic_position_image(...)
}

narrowness_index <- function(...) {
  # Computes narrowness index (perimeter / sqrt(area)) for polygon features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$narrowness_index(...)
}

natural_neighbour_interpolation <- function(...) {
  # Interpolates a raster from point samples using a Delaunay-neighbour weighted scheme.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$natural_neighbour_interpolation(...)
}

near <- function(...) {
  # Finds the nearest feature in a near layer and writes NEAR_FID and NEAR_DIST attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$near(...)
}

nearest_neighbour_interpolation <- function(...) {
  # Interpolates a raster from point samples by assigning each cell the nearest sample value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$nearest_neighbour_interpolation(...)
}

negate <- function(...) {
  # Negates each non-nodata raster cell value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$negate(...)
}

network_accessibility_metrics <- function(...) {
  # Computes accessibility indices for origin points based on reachability to destinations with optional impedance cutoffs and decay functions.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_accessibility_metrics(...)
}

network_centrality_metrics <- function(...) {
  # Computes baseline degree, closeness, and betweenness centrality metrics for network nodes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_centrality_metrics(...)
}

network_connected_components <- function(...) {
  # Assigns a connected-component ID to each line feature in a network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_connected_components(...)
}

network_node_degree <- function(...) {
  # Extracts network nodes from line features and computes node degree and node type.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_node_degree(...)
}

network_od_cost_matrix <- function(...) {
  # Computes origin-destination shortest-path costs over a line network and writes a CSV matrix.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_od_cost_matrix(...)
}

network_routes_from_od <- function(...) {
  # Builds route geometries for origin-destination point pairs over a line network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_routes_from_od(...)
}

network_service_area <- function(...) {
  # Computes reachable network nodes from origin points within a maximum network cost.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_service_area(...)
}

network_topology_audit <- function(...) {
  # Audits a line network for topology anomalies—disconnected components, dead ends, and degree anomalies—that cause routing failures.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$network_topology_audit(...)
}

new_raster_from_base_raster <- function(...) {
  # Creates a new raster using the extent, dimensions, and CRS of a base raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$new_raster_from_base_raster(...)
}

new_raster_from_base_vector <- function(...) {
  # Creates a new raster from a base vector extent and cell size, filled with an optional value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$new_raster_from_base_vector(...)
}

nibble <- function(...) {
  # Fills background regions using nearest-neighbour allocation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$nibble(...)
}

nnd_classification <- function(...) {
  # Performs nearest-normalized-distance classification with optional outlier rejection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$nnd_classification(...)
}

non_local_means_filter <- function(...) {
  # Performs non-local means denoising using patch similarity weighting.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$non_local_means_filter(...)
}

normal_vectors <- function(...) {
  # Estimates local point-cloud normals and stores them in point normals and RGB values.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$normal_vectors(...)
}

normalize_lidar <- function(...) {
  # Normalizes LiDAR z-values using a raster DTM so elevations become height above ground.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$normalize_lidar(...)
}

normalized_difference_index <- function(...) {
  # Computes (band1 - band2) / (band1 + band2) from a multiband raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$normalized_difference_index(...)
}

not_equal_to <- function(...) {
  # Tests whether two rasters are not equal on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$not_equal_to(...)
}

num_downslope_neighbours <- function(...) {
  # Counts the number of 8-neighbour cells lower than each DEM cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$num_downslope_neighbours(...)
}

num_inflowing_neighbours <- function(...) {
  # Counts the number of inflowing D8 neighbours for each DEM cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$num_inflowing_neighbours(...)
}

num_upslope_neighbours <- function(...) {
  # Counts the number of 8-neighbour cells higher than each DEM cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$num_upslope_neighbours(...)
}

od_sensitivity_analysis <- function(...) {
  # Computes OD shortest-path costs with impedance perturbations and outputs sensitivity statistics via Monte Carlo sampling.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$od_sensitivity_analysis(...)
}

olympic_filter <- function(...) {
  # Performs Olympic smoothing by averaging local values excluding min and max.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$olympic_filter(...)
}

opening <- function(...) {
  # Performs a morphological opening operation using a rectangular structuring element.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$opening(...)
}

openness <- function(...) {
  # Calculates Yokoyama et al. (2002) topographic openness from an input DEM. Returns positive (convex) and negative (concave) openness rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$openness(...)
}

otsu_thresholding <- function(...) {
  # Applies Otsu's automatic thresholding to create a binary raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$otsu_thresholding(...)
}

paired_sample_t_test <- function(...) {
  # Performs a paired-sample t-test on two rasters using paired valid cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$paired_sample_t_test(...)
}

panchromatic_sharpening <- function(...) {
  # Fuses multispectral and panchromatic rasters using Brovey or IHS methods.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$panchromatic_sharpening(...)
}

parallelepiped_classification <- function(...) {
  # Performs a supervised parallelepiped classification on multi-spectral rasters using polygon training data.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$parallelepiped_classification(...)
}

patch_orientation <- function(...) {
  # Calculates polygon orientation (degrees from north) using reduced major axis regression and appends ORIENT.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$patch_orientation(...)
}

pennock_landform_classification <- function(...) {
  # Classifies landform elements into seven Pennock et al. (1987) terrain classes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$pennock_landform_classification(...)
}

percent_elev_range <- function(...) {
  # Calculates local topographic position as percent of neighbourhood elevation range.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$percent_elev_range(...)
}

percent_equal_to <- function(...) {
  # Computes the fraction of rasters in a stack whose values equal the comparison raster at each cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$percent_equal_to(...)
}

percent_greater_than <- function(...) {
  # Computes the fraction of rasters in a stack whose values are greater than the comparison raster at each cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$percent_greater_than(...)
}

percent_less_than <- function(...) {
  # Computes the fraction of rasters in a stack whose values are less than the comparison raster at each cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$percent_less_than(...)
}

percentage_contrast_stretch <- function(...) {
  # Performs linear contrast stretch with percentile clipping.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$percentage_contrast_stretch(...)
}

percentile_filter <- function(...) {
  # Computes center-cell percentile rank in a moving window.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$percentile_filter(...)
}

perimeter_area_ratio <- function(...) {
  # Calculates polygon perimeter/area ratio and appends P_A_RATIO.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$perimeter_area_ratio(...)
}

phi_coefficient <- function(...) {
  # Performs binary classification agreement assessment using the phi coefficient.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$phi_coefficient(...)
}

pick_from_list <- function(...) {
  # Selects per-cell values from a raster stack using a zero-based position raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$pick_from_list(...)
}

piecewise_contrast_stretch <- function(...) {
  # Performs piecewise linear contrast stretching using user-specified breakpoints.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$piecewise_contrast_stretch(...)
}

plan_curvature <- function(...) {
  # Calculates plan (contour) curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$plan_curvature(...)
}

points_along_lines <- function(...) {
  # Creates regularly spaced point features along input line geometries.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$points_along_lines(...)
}

polygon_area <- function(...) {
  # Calculates polygon area and appends an AREA attribute field.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$polygon_area(...)
}

polygon_long_axis <- function(...) {
  # Maps the long axis of each polygon feature's minimum bounding box as line output.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$polygon_long_axis(...)
}

polygon_perimeter <- function(...) {
  # Calculates polygon perimeter and appends a PERIMETER attribute field.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$polygon_perimeter(...)
}

polygon_short_axis <- function(...) {
  # Maps the short axis of each polygon feature's minimum bounding box as line output.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$polygon_short_axis(...)
}

polygonize <- function(...) {
  # Creates polygons from closed input linework rings.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$polygonize(...)
}

polygons_to_lines <- function(...) {
  # Converts polygon and multipolygon features into linework tracing their boundaries.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$polygons_to_lines(...)
}

power <- function(...) {
  # Raises the first raster to the power of the second on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$power(...)
}

prewitt_filter <- function(...) {
  # Performs Prewitt edge detection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$prewitt_filter(...)
}

principal_component_analysis <- function(...) {
  # Performs PCA on a stack of rasters, returning component images and a JSON report.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$principal_component_analysis(...)
}

principal_curvature_direction <- function(...) {
  # Calculates the principal curvature direction angle (degrees).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$principal_curvature_direction(...)
}

print_geotiff_tags <- function(...) {
  # Produces a text report describing TIFF/GeoTIFF tags and key metadata for an input GeoTIFF-family raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$print_geotiff_tags(...)
}

profile <- function(...) {
  # Creates an HTML elevation profile plot for one or more input polyline features sampled from a surface raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$profile(...)
}

profile_curvature <- function(...) {
  # Calculates profile curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$profile_curvature(...)
}

prune_vector_streams <- function(...) {
  # Prunes vector stream network based on Shreve magnitude.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$prune_vector_streams(...)
}

qin_flow_accumulation <- function(...) {
  # Calculates Qin MFD flow accumulation from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$qin_flow_accumulation(...)
}

quantiles <- function(...) {
  # Transforms raster values into quantile classes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$quantiles(...)
}

quinn_flow_accumulation <- function(...) {
  # Calculates Quinn MFD flow accumulation from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$quinn_flow_accumulation(...)
}

radial_basis_function_interpolation <- function(...) {
  # Interpolates a raster from point samples using local radial-basis similarity weighting.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$radial_basis_function_interpolation(...)
}

radius_of_gyration <- function(...) {
  # Computes per-patch radius of gyration and maps values back to patch cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$radius_of_gyration(...)
}

raise_walls <- function(...) {
  # Raises DEM elevations along wall vectors and optionally breaches selected crossings.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raise_walls(...)
}

random_field <- function(...) {
  # Creates a raster containing standard normal random values.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_field(...)
}

random_forest_classification <- function(...) {
  # Performs supervised random forest classification on multi-band input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_forest_classification(...)
}

random_forest_classification_fit <- function(...) {
  # Fits a random forest classification model and returns serialized model bytes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_forest_classification_fit(...)
}

random_forest_classification_predict <- function(...) {
  # Applies a serialized random forest classification model to multi-band predictors.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_forest_classification_predict(...)
}

random_forest_regression <- function(...) {
  # Performs supervised random forest regression on multi-band input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_forest_regression(...)
}

random_forest_regression_fit <- function(...) {
  # Fits a random forest regression model and returns serialized model bytes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_forest_regression_fit(...)
}

random_forest_regression_predict <- function(...) {
  # Applies a serialized random forest regression model to multi-band predictors.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_forest_regression_predict(...)
}

random_points_in_polygon <- function(...) {
  # Generates random points uniformly within input polygon geometries.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_points_in_polygon(...)
}

random_sample <- function(...) {
  # Creates a raster containing randomly located sample cells with unique IDs.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$random_sample(...)
}

range_filter <- function(...) {
  # Computes a moving-window range (max-min) for each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$range_filter(...)
}

raster_area <- function(...) {
  # Estimates per-class raster polygon area in grid-cell or map units and writes class totals to each class cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_area(...)
}

raster_calculator <- function(...) {
  # Evaluates a mathematical expression on a list of input rasters cell-by-cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_calculator(...)
}

raster_cell_assignment <- function(...) {
  # Creates a raster derived from a base raster assigning row, column, x, or y values to each cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_cell_assignment(...)
}

raster_histogram <- function(...) {
  # Builds a fixed-bin histogram for valid raster cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_histogram(...)
}

raster_perimeter <- function(...) {
  # Estimates per-class raster polygon perimeter using an anti-aliasing lookup table and writes class totals to each class cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_perimeter(...)
}

raster_streams_to_vector <- function(...) {
  # Converts raster stream network to vector.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_streams_to_vector(...)
}

raster_summary_stats <- function(...) {
  # Computes basic summary statistics for valid raster cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_summary_stats(...)
}

raster_to_vector_lines <- function(...) {
  # Converts non-zero, non-nodata raster line cells into polyline vector features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_to_vector_lines(...)
}

raster_to_vector_points <- function(...) {
  # Converts non-zero, non-nodata cells in a raster into point features located at cell centres.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_to_vector_points(...)
}

raster_to_vector_polygons <- function(...) {
  # Converts non-zero, non-nodata raster regions into polygon vector features with FID and VALUE attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$raster_to_vector_polygons(...)
}

rasterize_streams <- function(...) {
  # Rasterizes vector stream network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rasterize_streams(...)
}

reciprocal <- function(...) {
  # Computes the reciprocal (1/x) of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$reciprocal(...)
}

reclass <- function(...) {
  # Reclassifies raster values using either ranges or exact assignment pairs.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$reclass(...)
}

reclass_equal_interval <- function(...) {
  # Reclassifies raster values into equal-width intervals over an optional value range.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$reclass_equal_interval(...)
}

recover_flightline_info <- function(...) {
  # Infers flightlines from GPS-time gaps and writes identifiers to point source ID, user data, and/or RGB.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$recover_flightline_info(...)
}

rectangular_grid_from_raster_base <- function(...) {
  # Creates a rectangular polygon grid covering a raster extent.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rectangular_grid_from_raster_base(...)
}

rectangular_grid_from_vector_base <- function(...) {
  # Creates a rectangular polygon grid covering a vector-layer bounding extent.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rectangular_grid_from_vector_base(...)
}

reinitialize_attribute_table <- function(...) {
  # Creates a copy of a vector layer with only a regenerated FID attribute.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$reinitialize_attribute_table(...)
}

related_circumscribing_circle <- function(...) {
  # Calculates 1 - (polygon area / smallest circumscribing circle area) and appends RC_CIRCLE.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$related_circumscribing_circle(...)
}

relative_aspect <- function(...) {
  # Calculates terrain aspect relative to a user-specified azimuth (0 to 180 degrees).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$relative_aspect(...)
}

relative_stream_power_index <- function(...) {
  # Calculates the relative stream power index from specific catchment area and slope.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$relative_stream_power_index(...)
}

relative_topographic_position <- function(...) {
  # Calculates RTP using neighbourhood min, mean, and max elevation values.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$relative_topographic_position(...)
}

remove_duplicates <- function(...) {
  # Removes duplicate LiDAR points using x/y and optionally z coordinates.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$remove_duplicates(...)
}

remove_off_terrain_objects <- function(...) {
  # Removes steep off-terrain objects from DEMs using white top-hat normalization, slope-constrained region growing, and local interpolation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$remove_off_terrain_objects(...)
}

remove_polygon_holes <- function(...) {
  # Removes interior rings from polygon features while preserving attributes.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$remove_polygon_holes(...)
}

remove_raster_polygon_holes <- function(...) {
  # Removes interior background holes (0 or nodata regions enclosed by foreground) from raster polygons.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$remove_raster_polygon_holes(...)
}

remove_short_streams <- function(...) {
  # Removes stream links shorter than minimum length.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$remove_short_streams(...)
}

remove_spurs <- function(...) {
  # Removes short spur artifacts from binary raster features by iterative pruning.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$remove_spurs(...)
}

rename_field <- function(...) {
  # Renames an attribute field in a vector layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rename_field(...)
}

repair_stream_vector_topology <- function(...) {
  # Repairs topology of vector stream network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$repair_stream_vector_topology(...)
}

reproject_vector <- function(...) {
  # Reprojects an input vector layer to a destination EPSG code.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$reproject_vector(...)
}

resample <- function(...) {
  # Resamples one or more input rasters to a base raster grid or to a user-defined output cell size.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$resample(...)
}

rescale_value_range <- function(...) {
  # Linearly rescales raster values into a target range.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rescale_value_range(...)
}

rgb_to_ihs <- function(...) {
  # Transforms red, green, blue band rasters (or a packed composite) to intensity, hue, and saturation components.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rgb_to_ihs(...)
}

rho8_flow_accum <- function(...) {
  # Calculates Rho8 flow accumulation from a DEM or Rho8 pointer raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rho8_flow_accum(...)
}

rho8_pointer <- function(...) {
  # Generates a Rho8 stochastic single-flow-direction pointer raster from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rho8_pointer(...)
}

ridge_and_valley_vectors <- function(...) {
  # Extracts ridge and valley centreline vectors from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ridge_and_valley_vectors(...)
}

ring_curvature <- function(...) {
  # Calculates ring curvature (squared flow-line twisting) from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ring_curvature(...)
}

river_centerlines <- function(...) {
  # Extracts river centerlines from water raster using medial axis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$river_centerlines(...)
}

roberts_cross_filter <- function(...) {
  # Performs Roberts Cross edge detection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$roberts_cross_filter(...)
}

root_mean_square_error <- function(...) {
  # Calculates RMSE and related accuracy statistics between two rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$root_mean_square_error(...)
}

rotor <- function(...) {
  # Calculates the rotor (flow-line twisting) from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$rotor(...)
}

round <- function(...) {
  # Rounds each raster cell to the nearest integer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$round(...)
}

route_calibrate <- function(...) {
  # Calibrates route start/end measures from control points with known measures.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_calibrate(...)
}

route_event_lines_from_layer <- function(...) {
  # Creates routed line events from an event vector layer using from/to measures.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_event_lines_from_layer(...)
}

route_event_lines_from_table <- function(...) {
  # Creates routed line events from a CSV event table and a route layer using from/to measures.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_event_lines_from_table(...)
}

route_event_merge <- function(...) {
  # Merges adjacent compatible route events.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_event_merge(...)
}

route_event_overlay <- function(...) {
  # Overlays two route event layers by interval overlap.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_event_overlay(...)
}

route_event_points_from_layer <- function(...) {
  # Creates routed point events from an event vector layer and a route layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_event_points_from_layer(...)
}

route_event_points_from_table <- function(...) {
  # Creates routed point events from a CSV event table and a route layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_event_points_from_table(...)
}

route_event_split <- function(...) {
  # Splits route events by per-route boundary measures.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_event_split(...)
}

route_measure_qa <- function(...) {
  # Diagnoses route-event measure gaps, overlaps, non-monotonic sequences, and duplicate measures.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_measure_qa(...)
}

route_recalibrate <- function(...) {
  # Recalibrates edited route measures from a reference route layer while preserving route measure continuity.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$route_recalibrate(...)
}

ruggedness_index <- function(...) {
  # Calculates the terrain ruggedness index (TRI) after Riley et al. (1999).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$ruggedness_index(...)
}

savitzky_golay_2d_filter <- function(...) {
  # Performs 2D Savitzky-Golay smoothing.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$savitzky_golay_2d_filter(...)
}

scharr_filter <- function(...) {
  # Performs Scharr edge detection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$scharr_filter(...)
}

sediment_transport_index <- function(...) {
  # Calculates the sediment transport index (LS factor) from specific catchment area and slope.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sediment_transport_index(...)
}

select_by_location <- function(...) {
  # Extracts target features that satisfy a spatial relationship to query features.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$select_by_location(...)
}

select_tiles_by_polygon <- function(...) {
  # Copies LiDAR tiles from an input directory to an output directory when tile sample points overlap polygon geometries.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$select_tiles_by_polygon(...)
}

set_nodata_value <- function(...) {
  # Sets a raster nodata value and maps existing nodata cells to the specified background value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$set_nodata_value(...)
}

shape_complexity_index_raster <- function(...) {
  # Computes raster patch shape complexity from horizontal/vertical transition frequency normalized by patch span.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$shape_complexity_index_raster(...)
}

shape_complexity_index_vector <- function(...) {
  # Computes shape complexity index for vector polygon features using normalized form factor.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$shape_complexity_index_vector(...)
}

shape_index <- function(...) {
  # Calculates the shape index surface form descriptor from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$shape_index(...)
}

shortest_path_network <- function(...) {
  # Finds the shortest path between start and end coordinates over a line network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$shortest_path_network(...)
}

shreve_stream_magnitude <- function(...) {
  # Calculates Shreve stream magnitude.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$shreve_stream_magnitude(...)
}

sieve <- function(...) {
  # Removes small isolated patches below a cell-count threshold.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sieve(...)
}

sigmoidal_contrast_stretch <- function(...) {
  # Performs sigmoidal contrast stretching using gain and cutoff.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sigmoidal_contrast_stretch(...)
}

simplify_features <- function(...) {
  # Simplifies vector geometries using Douglas-Peucker tolerance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$simplify_features(...)
}

sin <- function(...) {
  # Computes the sine of each raster cell value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sin(...)
}

singlepart_to_multipart <- function(...) {
  # Merges single-part features into multi-part features, grouped by an optional categorical field.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$singlepart_to_multipart(...)
}

sinh <- function(...) {
  # Computes the hyperbolic sine of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sinh(...)
}

sink <- function(...) {
  # Identifies cells that belong to topographic depressions in a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sink(...)
}

sky_view_factor <- function(...) {
  # Calculates the proportion of visible sky from a DEM/DSM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sky_view_factor(...)
}

slope <- function(...) {
  # Calculates slope gradient from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$slope(...)
}

slope_vs_elev_plot <- function(...) {
  # Creates an HTML slope-vs-elevation analysis chart for one or more DEMs.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$slope_vs_elev_plot(...)
}

smooth_vectors <- function(...) {
  # Smooths polyline or polygon vectors using a moving-average filter.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$smooth_vectors(...)
}

snap_endnodes <- function(...) {
  # Snaps nearby polyline endpoints to a shared location within a tolerance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$snap_endnodes(...)
}

snap_pour_points <- function(...) {
  # Snaps pour points to the highest flow-accumulation cell within a search distance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$snap_pour_points(...)
}

sobel_filter <- function(...) {
  # Performs Sobel edge detection.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sobel_filter(...)
}

sort_lidar <- function(...) {
  # Sorts points by one or more LiDAR properties, with optional bin sizes per criterion.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sort_lidar(...)
}

spatial_join <- function(...) {
  # Joins attributes from a join layer onto target features using a spatial predicate.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$spatial_join(...)
}

spherical_std_dev_of_normals <- function(...) {
  # Calculates spherical standard deviation of local surface normals.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$spherical_std_dev_of_normals(...)
}

split_colour_composite <- function(...) {
  # Splits a packed RGB colour composite into separate red, green, and blue single-band rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$split_colour_composite(...)
}

split_lidar <- function(...) {
  # Splits LiDAR points into multiple output files based on a grouping criterion.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$split_lidar(...)
}

split_vector_lines <- function(...) {
  # Splits each polyline feature into segments of a maximum specified length.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$split_vector_lines(...)
}

split_with_lines <- function(...) {
  # Splits input polylines using intersection points from a split line layer.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$split_with_lines(...)
}

sqrt <- function(...) {
  # Computes the square-root of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sqrt(...)
}

square <- function(...) {
  # Squares each raster cell value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$square(...)
}

standard_deviation_contrast_stretch <- function(...) {
  # Performs linear contrast stretch using mean plus/minus a standard deviation multiplier.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$standard_deviation_contrast_stretch(...)
}

standard_deviation_filter <- function(...) {
  # Computes a moving-window standard deviation for each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$standard_deviation_filter(...)
}

standard_deviation_of_slope <- function(...) {
  # Calculates local standard deviation of slope as a terrain roughness metric.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$standard_deviation_of_slope(...)
}

standard_deviation_overlay <- function(...) {
  # Computes the per-cell standard deviation across a raster stack, propagating NoData if any input cell is NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$standard_deviation_overlay(...)
}

stochastic_depression_analysis <- function(...) {
  # Runs Monte Carlo DEM perturbations and estimates depression-membership probability.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$stochastic_depression_analysis(...)
}

strahler_order_basins <- function(...) {
  # Delineates watershed basins labelled by the Horton-Strahler order of their draining stream link.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$strahler_order_basins(...)
}

strahler_stream_order <- function(...) {
  # Assigns Strahler stream order to stream cells.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$strahler_stream_order(...)
}

stream_link_class <- function(...) {
  # Classifies stream links as interior, exterior, or source.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$stream_link_class(...)
}

stream_link_identifier <- function(...) {
  # Assigns unique ID to each stream link.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$stream_link_identifier(...)
}

stream_link_length <- function(...) {
  # Calculates total length for each stream link.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$stream_link_length(...)
}

stream_link_slope <- function(...) {
  # Calculates average slope for each stream link.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$stream_link_slope(...)
}

stream_slope_continuous <- function(...) {
  # Calculates slope value for each stream cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$stream_slope_continuous(...)
}

subbasins <- function(...) {
  # Identifies the catchment area of each stream link (sub-basins) in a D8 stream network.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$subbasins(...)
}

subtract <- function(...) {
  # Subtracts the second raster from the first on a cell-by-cell basis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$subtract(...)
}

sum_overlay <- function(...) {
  # Computes the per-cell sum across a raster stack, propagating NoData if any input cell is NoData.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$sum_overlay(...)
}

surface_area_ratio <- function(...) {
  # Calculates the ratio of 3D surface area to planimetric area using the Jenness (2004) method.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$surface_area_ratio(...)
}

svm_classification <- function(...) {
  # Performs supervised support-vector-machine classification on multi-band input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$svm_classification(...)
}

svm_regression <- function(...) {
  # Performs supervised support-vector-machine regression on multi-band input rasters.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$svm_regression(...)
}

symmetrical_difference <- function(...) {
  # Computes non-overlapping polygon regions from input and overlay layers.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$symmetrical_difference(...)
}

tan <- function(...) {
  # Computes the tangent of each raster cell value.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$tan(...)
}

tangential_curvature <- function(...) {
  # Calculates tangential curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$tangential_curvature(...)
}

tanh <- function(...) {
  # Computes the hyperbolic tangent of each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$tanh(...)
}

thicken_raster_line <- function(...) {
  # Thickens diagonal raster line segments to prevent diagonal leak-through.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$thicken_raster_line(...)
}

time_in_daylight <- function(...) {
  # Calculates the proportion of daytime each cell is illuminated (not in terrain/object shadow).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$time_in_daylight(...)
}

tin_interpolation <- function(...) {
  # Interpolates a raster from point samples using Delaunay triangulation and planar interpolation within each triangle.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$tin_interpolation(...)
}

to_degrees <- function(...) {
  # Converts each raster cell from radians to degrees.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$to_degrees(...)
}

to_radians <- function(...) {
  # Converts each raster cell from degrees to radians.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$to_radians(...)
}

tophat_transform <- function(...) {
  # Performs a white or black morphological top-hat transform.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$tophat_transform(...)
}

topological_stream_order <- function(...) {
  # Assigns topological stream order based on link count.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$topological_stream_order(...)
}

topology_rule_autofix <- function(...) {
  # Automatically applies safe, auditable fixes to topology violations detected by topology_rule_validate.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$topology_rule_autofix(...)
}

topology_rule_validate <- function(...) {
  # Validates vector topology against rule-set checks (self-intersection, overlap, gaps, dangles, point coverage, endpoint snapping) and emits feature-level violations.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$topology_rule_validate(...)
}

topology_validation_report <- function(...) {
  # Audits a vector layer for topology issues and writes a per-feature CSV report.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$topology_validation_report(...)
}

total_curvature <- function(...) {
  # Calculates total curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$total_curvature(...)
}

total_filter <- function(...) {
  # Computes a moving-window total for each raster cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$total_filter(...)
}

trace_downslope_flowpaths <- function(...) {
  # Marks D8 flowpaths initiated from seed points until no-flow or grid edge.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$trace_downslope_flowpaths(...)
}

travelling_salesman_problem <- function(...) {
  # Finds approximate solutions to the travelling salesman problem (TSP) using 2-opt heuristics. Given a set of point locations, identifies the shortest route connecting all points.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$travelling_salesman_problem(...)
}

trend_surface <- function(...) {
  # Fits a polynomial trend surface to a raster using least-squares regression.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$trend_surface(...)
}

trend_surface_vector_points <- function(...) {
  # Fits a polynomial trend surface to vector point data using least-squares regression.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$trend_surface_vector_points(...)
}

tributary_identifier <- function(...) {
  # Assigns unique ID to each tributary.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$tributary_identifier(...)
}

truncate <- function(...) {
  # Truncates each raster cell value to its integer part.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$truncate(...)
}

turning_bands_simulation <- function(...) {
  # Creates a spatially-autocorrelated random field using the turning bands algorithm.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$turning_bands_simulation(...)
}

two_sample_ks_test <- function(...) {
  # Performs a two-sample Kolmogorov-Smirnov test on two raster value distributions.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$two_sample_ks_test(...)
}

union <- function(...) {
  # Dissolves combined input and overlay polygons into a unified polygon coverage.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$union(...)
}

unnest_basins <- function(...) {
  # Creates one basin raster per pour-point nesting level from a D8 pointer grid.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$unnest_basins(...)
}

unsharp_masking <- function(...) {
  # Performs edge-enhancing unsharp masking.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$unsharp_masking(...)
}

unsphericity <- function(...) {
  # Calculates the unsphericity curvature (half the difference of principal curvatures) from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$unsphericity(...)
}

update_nodata_cells <- function(...) {
  # Assigns NoData cells in input1 from corresponding valid cells in input2.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$update_nodata_cells(...)
}

upslope_depression_storage <- function(...) {
  # Maps mean upslope depression-storage depth by routing depression depth over a conditioned DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$upslope_depression_storage(...)
}

user_defined_weights_filter <- function(...) {
  # Applies a user-defined convolution kernel.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$user_defined_weights_filter(...)
}

vector_hex_binning <- function(...) {
  # Aggregates point features into hexagonal bins, counting points per hex cell.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vector_hex_binning(...)
}

vector_lines_to_raster <- function(...) {
  # Rasterizes line and polygon boundary geometries to a raster grid.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vector_lines_to_raster(...)
}

vector_points_to_raster <- function(...) {
  # Rasterizes point or multipoint vectors to a grid using a selected assignment operation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vector_points_to_raster(...)
}

vector_polygons_to_raster <- function(...) {
  # Rasterizes polygon vectors to a grid, supporting attribute-driven burn values.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vector_polygons_to_raster(...)
}

vector_stream_network_analysis <- function(...) {
  # Comprehensive vector stream network analysis.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vector_stream_network_analysis(...)
}

vector_summary_statistics <- function(...) {
  # Computes grouped summary statistics for a numeric field and writes the result to CSV.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vector_summary_statistics(...)
}

vehicle_routing_cvrp <- function(...) {
  # Builds capacity-constrained delivery routes from depot and stop points using deterministic greedy construction with optional local optimization.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vehicle_routing_cvrp(...)
}

vehicle_routing_pickup_delivery <- function(...) {
  # Builds paired pickup-delivery routes with precedence and capacity constraints using a deterministic nearest-neighbour baseline.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vehicle_routing_pickup_delivery(...)
}

vehicle_routing_vrptw <- function(...) {
  # Builds capacity-constrained routes with time-window diagnostics using deterministic feasible-candidate scoring with optional nearest-neighbour baseline behavior.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vehicle_routing_vrptw(...)
}

vertical_excess_curvature <- function(...) {
  # Calculates vertical excess curvature from a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$vertical_excess_curvature(...)
}

viewshed <- function(...) {
  # Computes station visibility counts from point stations over a DEM.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$viewshed(...)
}

visibility_index <- function(...) {
  # Calculates a topography-based visibility index from sampled viewsheds.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$visibility_index(...)
}

voronoi_diagram <- function(...) {
  # Creates Voronoi (Thiessen) polygons from input point locations.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$voronoi_diagram(...)
}

watershed <- function(...) {
  # Delineates watersheds from a D8 pointer and vector pour points.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$watershed(...)
}

watershed_from_raster_pour_points <- function(...) {
  # Delineates watersheds from a D8 pointer and a raster of pour-point outlet IDs.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$watershed_from_raster_pour_points(...)
}

weighted_overlay <- function(...) {
  # Combines factor rasters using normalized weights, optional cost flags, and optional binary constraints.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$weighted_overlay(...)
}

weighted_sum <- function(...) {
  # Computes a weighted sum across a raster stack after normalizing weights to sum to one.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$weighted_sum(...)
}

wetness_index <- function(...) {
  # Calculates the topographic wetness index ln(SCA / tan(slope)).
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$wetness_index(...)
}

wiener_filter <- function(...) {
  # Performs adaptive Wiener denoising using local mean and variance.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$wiener_filter(...)
}

wilcoxon_signed_rank_test <- function(...) {
  # Performs a Wilcoxon signed-rank test on paired raster differences.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$wilcoxon_signed_rank_test(...)
}

write_function_memory_insertion <- function(...) {
  # Creates a packed RGB change-visualization composite from two or three single-band dates.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$write_function_memory_insertion(...)
}

z_scores <- function(...) {
  # Standardizes raster values to z-scores using global mean and standard deviation.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$z_scores(...)
}

zonal_statistics <- function(...) {
  # Summarises the values of a data raster within zones defined by a feature raster.
  session <- wbw_make_session(include_pro = FALSE, tier = "open")
  session$zonal_statistics(...)
}

