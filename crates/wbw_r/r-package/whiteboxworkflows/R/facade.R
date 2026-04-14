wbw_build_session <- function(floating_license_id = NULL,
                           include_pro = NULL,
                           tier = "open",
                           signed_entitlement_json = NULL,
                           entitlement_file = NULL,
                           public_key_kid = NULL,
                           public_key_b64url = NULL,
                           provider_url = NULL,
                           machine_id = NULL,
                           customer_id = NULL) {
  if (!is.null(signed_entitlement_json) && !is.null(entitlement_file)) {
    stop("Provide either signed_entitlement_json or entitlement_file, not both.", call. = FALSE)
  }

  if ((!is.null(signed_entitlement_json) || !is.null(entitlement_file)) &&
      (is.null(public_key_kid) || is.null(public_key_b64url))) {
    stop("Entitlement-based startup requires public_key_kid and public_key_b64url.", call. = FALSE)
  }

  if (!is.null(signed_entitlement_json)) {
    resolved_include_pro <- if (is.null(include_pro)) FALSE else include_pro
    session <- wbw_make_entitlement_session(
      signed_entitlement_json = signed_entitlement_json,
      public_key_kid = public_key_kid,
      public_key_b64url = public_key_b64url,
      include_pro = resolved_include_pro,
      fallback_tier = tier
    )
  } else if (!is.null(entitlement_file)) {
    resolved_include_pro <- if (is.null(include_pro)) FALSE else include_pro
    session <- wbw_make_entitlement_file_session(
      entitlement_file = entitlement_file,
      public_key_kid = public_key_kid,
      public_key_b64url = public_key_b64url,
      include_pro = resolved_include_pro,
      fallback_tier = tier
    )
  } else {
    session <- wbw_make_session(
      floating_license_id = floating_license_id,
      include_pro = include_pro,
      tier = tier,
      provider_url = provider_url,
      machine_id = machine_id,
      customer_id = customer_id
    )

    resolved_include_pro <- if (is.null(include_pro)) !is.null(floating_license_id) else include_pro
    session$run_tool_with_progress <- function(tool_id, args = list()) {
      args_json <- wbw_args_to_json(args)
      if (!is.null(floating_license_id) &&
          is.loaded("wrap__run_tool_json_with_progress_floating_license_id_options")) {
        out_json <- run_tool_json_with_progress_floating_license_id_options(
          tool_id,
          args_json,
          floating_license_id,
          resolved_include_pro,
          tier,
          provider_url,
          machine_id,
          customer_id
        )
        return(jsonlite::fromJSON(out_json, simplifyVector = FALSE))
      }

      if (is.null(floating_license_id) && is.loaded("wrap__run_tool_json_with_progress_options")) {
        out_json <- run_tool_json_with_progress_options(tool_id, args_json, resolved_include_pro, tier)
        return(jsonlite::fromJSON(out_json, simplifyVector = FALSE))
      }

      wbw_progress_result_fallback(tool_id, session$run_tool(tool_id, args))
    }
  }

  session$write_raster <- function(raster,
                                   output_path,
                                   options = NULL,
                                   compress = NULL,
                                   strict_format_options = FALSE) {
    wbw_write_raster(
      raster = raster,
      output_path = output_path,
      options = options,
      compress = compress,
      strict_format_options = strict_format_options,
      session = session
    )
  }

  session$write_rasters <- function(rasters,
                                    output_paths,
                                    options = NULL,
                                    compress = NULL,
                                    strict_format_options = FALSE) {
    wbw_write_rasters(
      rasters = rasters,
      output_paths = output_paths,
      options = options,
      compress = compress,
      strict_format_options = strict_format_options,
      session = session
    )
  }

  session$write_vector <- function(vector,
                                   output_path,
                                   options = NULL,
                                   strict_format_options = FALSE) {
    wbw_write_vector(
      vector = vector,
      output_path = output_path,
      options = options,
      strict_format_options = strict_format_options,
      session = session
    )
  }

  session$read_vector <- function(path, options = NULL, strict_format_options = FALSE) {
    wbw_read_vector(
      path = path,
      options = options,
      strict_format_options = strict_format_options,
      session = session
    )
  }

  session$projection_to_ogc_wkt <- function(epsg) {
    wbw_projection_to_ogc_wkt(epsg)
  }

  session$projection_identify_epsg <- function(crs_text) {
    wbw_projection_identify_epsg(crs_text)
  }

  session$projection_reproject_points <- function(points, src_epsg, dst_epsg) {
    wbw_projection_reproject_points(points, src_epsg, dst_epsg)
  }

  session$projection_reproject_point <- function(x, y, src_epsg, dst_epsg) {
    wbw_projection_reproject_point(x, y, src_epsg, dst_epsg)
  }

  session$topology_intersects_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_intersects_wkt(a_wkt, b_wkt)
  }

  session$topology_contains_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_contains_wkt(a_wkt, b_wkt)
  }

  session$topology_within_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_within_wkt(a_wkt, b_wkt)
  }

  session$topology_touches_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_touches_wkt(a_wkt, b_wkt)
  }

  session$topology_disjoint_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_disjoint_wkt(a_wkt, b_wkt)
  }

  session$topology_crosses_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_crosses_wkt(a_wkt, b_wkt)
  }

  session$topology_overlaps_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_overlaps_wkt(a_wkt, b_wkt)
  }

  session$topology_covers_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_covers_wkt(a_wkt, b_wkt)
  }

  session$topology_covered_by_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_covered_by_wkt(a_wkt, b_wkt)
  }

  session$topology_relate_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_relate_wkt(a_wkt, b_wkt)
  }

  session$topology_distance_wkt <- function(a_wkt, b_wkt) {
    wbw_topology_distance_wkt(a_wkt, b_wkt)
  }

  session$topology_vector_feature_relation <- function(a_vector, a_feature_index, b_vector, b_feature_index) {
    wbw_topology_vector_feature_relation(a_vector, a_feature_index, b_vector, b_feature_index)
  }

  session$topology_is_valid_polygon_wkt <- function(wkt) {
    wbw_topology_is_valid_polygon_wkt(wkt)
  }

  session$topology_make_valid_polygon_wkt <- function(wkt, epsilon = 1e-9) {
    wbw_topology_make_valid_polygon_wkt(wkt, epsilon = epsilon)
  }

  session$topology_buffer_wkt <- function(wkt, distance) {
    wbw_topology_buffer_wkt(wkt, distance)
  }

  session$analyze_multimodal_od_scenarios <- function(input,
                                                       origins,
                                                       destinations,
                                                       output,
                                                       mode_field = "MODE",
                                                       allowed_modes = NULL,
                                                       mode_speed_overrides = NULL,
                                                       transfer_penalty = NULL,
                                                       edge_cost_field = NULL,
                                                       max_snap_distance = NULL,
                                                       scenario_bundle_csv = NULL,
                                                       temporal_cost_profile = NULL,
                                                       departure_time = NULL,
                                                       temporal_mode = NULL,
                                                       parallel_execution = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        input = input,
        origins = origins,
        destinations = destinations,
        output = output,
        mode_field = mode_field,
        allowed_modes = allowed_modes,
        mode_speed_overrides = mode_speed_overrides,
        transfer_penalty = transfer_penalty,
        edge_cost_field = edge_cost_field,
        max_snap_distance = max_snap_distance,
        scenario_bundle_csv = scenario_bundle_csv,
        temporal_cost_profile = temporal_cost_profile,
        departure_time = departure_time,
        temporal_mode = temporal_mode,
        parallel_execution = parallel_execution
      )
    )
    session$run_tool("multimodal_od_cost_matrix", args)
  }

  session$export_multimodal_routes_for_od_pairs <- function(input,
                                                             origins,
                                                             destinations,
                                                             output,
                                                             mode_field = "MODE",
                                                             allowed_modes = NULL,
                                                             mode_speed_overrides = NULL,
                                                             transfer_penalty = NULL,
                                                             edge_cost_field = NULL,
                                                             max_snap_distance = NULL,
                                                             scenario_bundle_csv = NULL,
                                                             temporal_cost_profile = NULL,
                                                             departure_time = NULL,
                                                             temporal_mode = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        input = input,
        origins = origins,
        destinations = destinations,
        output = output,
        mode_field = mode_field,
        allowed_modes = allowed_modes,
        mode_speed_overrides = mode_speed_overrides,
        transfer_penalty = transfer_penalty,
        edge_cost_field = edge_cost_field,
        max_snap_distance = max_snap_distance,
        scenario_bundle_csv = scenario_bundle_csv,
        temporal_cost_profile = temporal_cost_profile,
        departure_time = departure_time,
        temporal_mode = temporal_mode
      )
    )
    session$run_tool("multimodal_routes_from_od", args)
  }

  session$compute_network_accessibility <- function(input,
                                                    origins,
                                                    destinations,
                                                    output,
                                                    edge_cost_field = NULL,
                                                    max_snap_distance = NULL,
                                                    impedance_cutoff = NULL,
                                                    decay_function = NULL,
                                                    decay_parameter = NULL,
                                                    parallel_execution = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        input = input,
        origins = origins,
        destinations = destinations,
        output = output,
        edge_cost_field = edge_cost_field,
        max_snap_distance = max_snap_distance,
        impedance_cutoff = impedance_cutoff,
        decay_function = decay_function,
        decay_parameter = decay_parameter,
        parallel_execution = parallel_execution
      )
    )
    session$run_tool("network_accessibility_metrics", args)
  }

  session$analyze_od_cost_sensitivity <- function(input,
                                                  origins,
                                                  destinations,
                                                  output,
                                                  edge_cost_field = NULL,
                                                  max_snap_distance = NULL,
                                                  impedance_disturbance_range = NULL,
                                                  monte_carlo_samples = NULL,
                                                  parallel_execution = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        input = input,
        origins = origins,
        destinations = destinations,
        output = output,
        edge_cost_field = edge_cost_field,
        max_snap_distance = max_snap_distance,
        impedance_disturbance_range = impedance_disturbance_range,
        monte_carlo_samples = monte_carlo_samples,
        parallel_execution = parallel_execution
      )
    )
    session$run_tool("od_sensitivity_analysis", args)
  }

  session$lidar_change_and_disturbance_analysis <- function(baseline_tiles,
                                                            monitor_tiles,
                                                            resolution = NULL,
                                                            min_change_m = NULL,
                                                            output_prefix = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        baseline_tiles = baseline_tiles,
        monitor_tiles = monitor_tiles,
        resolution = resolution,
        min_change_m = min_change_m,
        output_prefix = output_prefix
      )
    )
    session$run_tool("lidar_change_and_disturbance_analysis", args)
  }

  session$sidewalk_vegetation_accessibility_monitoring <- function(lidar_tiles,
                                                                    sidewalks,
                                                                    sidewalks_epsg = NULL,
                                                                    resolution = NULL,
                                                                    segment_length_m = NULL,
                                                                    clearance_height_m = NULL,
                                                                    buffer_distance_m = NULL,
                                                                    output_prefix = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        lidar_tiles = lidar_tiles,
        sidewalks = sidewalks,
        sidewalks_epsg = sidewalks_epsg,
        resolution = resolution,
        segment_length_m = segment_length_m,
        clearance_height_m = clearance_height_m,
        buffer_distance_m = buffer_distance_m,
        output_prefix = output_prefix
      )
    )
    session$run_tool("sidewalk_vegetation_accessibility_monitoring", args)
  }

  session$terrain_constraint_and_conflict_analysis <- function(dem,
                                                                wetness = NULL,
                                                                flood_risk = NULL,
                                                                landcover_penalty = NULL,
                                                                slope_limit_deg = NULL,
                                                                output_prefix = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        dem = dem,
        wetness = wetness,
        flood_risk = flood_risk,
        landcover_penalty = landcover_penalty,
        slope_limit_deg = slope_limit_deg,
        output_prefix = output_prefix
      )
    )
    session$run_tool("terrain_constraint_and_conflict_analysis", args)
  }

  session$terrain_constructability_and_cost_analysis <- function(dem,
                                                                 existing_conflict = NULL,
                                                                 wetness = NULL,
                                                                 access_cost = NULL,
                                                                 output_prefix = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        dem = dem,
        existing_conflict = existing_conflict,
        wetness = wetness,
        access_cost = access_cost,
        output_prefix = output_prefix
      )
    )
    session$run_tool("terrain_constructability_and_cost_analysis", args)
  }

  session$in_season_crop_stress_intervention_planning <- function(ndvi,
                                                                   canopy_temperature = NULL,
                                                                   soil_moisture = NULL,
                                                                   output_prefix = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        ndvi = ndvi,
        canopy_temperature = canopy_temperature,
        soil_moisture = soil_moisture,
        output_prefix = output_prefix
      )
    )
    session$run_tool("in_season_crop_stress_intervention_planning", args)
  }

  session$field_trafficability_and_operation_planning <- function(dem,
                                                                   soil_moisture,
                                                                   rainfall_forecast = NULL,
                                                                   output_prefix = NULL) {
    args <- Filter(
      Negate(is.null),
      list(
        dem = dem,
        soil_moisture = soil_moisture,
        rainfall_forecast = rainfall_forecast,
        output_prefix = output_prefix
      )
    )
    session$run_tool("field_trafficability_and_operation_planning", args)
  }

  class(session) <- unique(c("wbw_session", class(session)))
  session
}

#' @export
wbw_session <- function(floating_license_id = NULL,
                        include_pro = NULL,
                        tier = "open",
                        signed_entitlement_json = NULL,
                        entitlement_file = NULL,
                        public_key_kid = NULL,
                        public_key_b64url = NULL,
                        provider_url = NULL,
                        machine_id = NULL,
                        customer_id = NULL) {
  wbw_build_session(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    signed_entitlement_json = signed_entitlement_json,
    entitlement_file = entitlement_file,
    public_key_kid = public_key_kid,
    public_key_b64url = public_key_b64url,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}

whitebox_tools <- function(...) {
  stop(
    "whitebox_tools() was removed in Phase 4. Use wbw_session() instead.",
    call. = FALSE
  )
}

#' @export
print.wbw_session <- function(x, ...) {
  tools <- x$list_tools()
  n <- length(tools)
  cat("<wbw_session>", n, "visible tool(s)\n")
  if (n > 0L) {
    ids <- vapply(tools, function(t) t$id %||% "", character(1))
    preview <- stats::na.omit(ids[seq_len(min(10L, length(ids)))])
    if (length(preview) > 0L) {
      cat("Sample:", paste(preview, collapse = ", "), "\n")
    }
  }
  invisible(x)
}

#' @export
print.wbw_raster <- function(x, ...) {
  meta <- x$metadata()
  cat("<wbw_raster>", basename(x$path), "\n")
  cat("Dimensions:", meta$rows, "rows x", meta$columns, "cols x", meta$bands, "band(s)\n")
  if (!is.null(meta$crs) && nzchar(meta$crs)) {
    cat("CRS:", meta$crs, "\n")
  }
  invisible(x)
}

#' @export
print.wbw_vector <- function(x, ...) {
  meta <- x$metadata()
  cat("<wbw_vector>", basename(x$path), "\n")
  cat("Geometry:", meta$geometry_type, "\n")
  cat("Features:", meta$feature_count, "\n")
  if (!is.null(meta$crs) && nzchar(meta$crs)) {
    cat("CRS:", meta$crs, "\n")
  }
  invisible(x)
}

#' @export
print.wbw_lidar <- function(x, ...) {
  meta <- x$metadata()
  cat("<wbw_lidar>", basename(x$path), "\n")
  cat("Format:", meta$format, "\n")
  cat("Points:", meta$point_count, "\n")
  if (!is.null(meta$crs_epsg) && !is.na(meta$crs_epsg)) {
    cat("CRS EPSG:", meta$crs_epsg, "\n")
  }
  invisible(x)
}

#' @export
print.wbw_sensor_bundle <- function(x, ...) {
  meta <- x$metadata()
  cat("<wbw_sensor_bundle>", basename(x$bundle_root), "\n")
  cat("Family:", meta$family, "\n")
  if (!is.null(meta$acquisition_datetime_utc) && nzchar(meta$acquisition_datetime_utc)) {
    cat("Acquired:", meta$acquisition_datetime_utc, "\n")
  }
  invisible(x)
}

`%||%` <- function(x, y) {
  if (is.null(x) || length(x) == 0L) y else x
}

wbw_output_key_is_metadata <- function(key) {
  key %in% c("__wbw_type__", "active_band", "band", "cells_processed", "path")
}

#' Convert EPSG code to OGC WKT text.
#'
#' @export
wbw_projection_to_ogc_wkt <- function(epsg) {
  epsg_int <- as.integer(epsg)
  if (is.na(epsg_int) || epsg_int <= 0L) {
    stop("epsg must be a positive integer.", call. = FALSE)
  }
  projection_to_ogc_wkt(epsg_int)
}

#' Identify EPSG code from CRS/WKT text.
#'
#' Returns `NULL` when no EPSG match is available.
#'
#' @export
wbw_projection_identify_epsg <- function(crs_text) {
  if (!is.character(crs_text) || length(crs_text) != 1L || !nzchar(crs_text)) {
    stop("crs_text must be a non-empty string.", call. = FALSE)
  }
  epsg <- projection_identify_epsg(crs_text)
  if (is.null(epsg) || length(epsg) == 0L) {
    return(NULL)
  }
  as.integer(epsg)
}

#' Reproject XY points between EPSG codes.
#'
#' `points` may be a data.frame with `x` and `y` columns, or a list of
#' point lists containing numeric `x` and `y` entries.
#'
#' @export
wbw_projection_reproject_points <- function(points, src_epsg, dst_epsg) {
  src_epsg_int <- as.integer(src_epsg)
  dst_epsg_int <- as.integer(dst_epsg)
  if (is.na(src_epsg_int) || src_epsg_int <= 0L || is.na(dst_epsg_int) || dst_epsg_int <= 0L) {
    stop("src_epsg and dst_epsg must be positive integers.", call. = FALSE)
  }

  points_payload <- NULL
  if (is.data.frame(points)) {
    if (!("x" %in% names(points)) || !("y" %in% names(points))) {
      stop("points data.frame must contain 'x' and 'y' columns.", call. = FALSE)
    }
    points_payload <- lapply(seq_len(nrow(points)), function(i) {
      list(x = as.numeric(points$x[[i]]), y = as.numeric(points$y[[i]]))
    })
  } else if (is.list(points)) {
    points_payload <- lapply(points, function(pt) {
      if (!is.list(pt) || is.null(pt$x) || is.null(pt$y)) {
        stop("each point must contain numeric 'x' and 'y'.", call. = FALSE)
      }
      list(x = as.numeric(pt$x), y = as.numeric(pt$y))
    })
  } else {
    stop("points must be a data.frame or list of points.", call. = FALSE)
  }

  points_json <- jsonlite::toJSON(points_payload, auto_unbox = TRUE, null = "null")
  out_json <- projection_reproject_points_json(points_json, src_epsg_int, dst_epsg_int)
  out <- jsonlite::fromJSON(out_json, simplifyVector = TRUE)
  as.data.frame(out)
}

#' Reproject a single XY point between EPSG codes.
#'
#' @export
wbw_projection_reproject_point <- function(x, y, src_epsg, dst_epsg) {
  src_epsg_int <- as.integer(src_epsg)
  dst_epsg_int <- as.integer(dst_epsg)
  if (is.na(src_epsg_int) || src_epsg_int <= 0L || is.na(dst_epsg_int) || dst_epsg_int <= 0L) {
    stop("src_epsg and dst_epsg must be positive integers.", call. = FALSE)
  }
  out_json <- projection_reproject_point_json(as.numeric(x), as.numeric(y), src_epsg_int, dst_epsg_int)
  jsonlite::fromJSON(out_json, simplifyVector = TRUE)
}

#' Return whether two WKT geometries intersect.
#'
#' @export
wbw_topology_intersects_wkt <- function(a_wkt, b_wkt) {
  topology_intersects_wkt(a_wkt, b_wkt)
}

#' Return whether WKT geometry A contains geometry B.
#'
#' @export
wbw_topology_contains_wkt <- function(a_wkt, b_wkt) {
  topology_contains_wkt(a_wkt, b_wkt)
}

#' Return whether WKT geometry A is within geometry B.
#'
#' @export
wbw_topology_within_wkt <- function(a_wkt, b_wkt) {
  topology_within_wkt(a_wkt, b_wkt)
}

#' Return whether two WKT geometries touch.
#'
#' @export
wbw_topology_touches_wkt <- function(a_wkt, b_wkt) {
  topology_touches_wkt(a_wkt, b_wkt)
}

#' Return whether two WKT geometries are disjoint.
#'
#' @export
wbw_topology_disjoint_wkt <- function(a_wkt, b_wkt) {
  topology_disjoint_wkt(a_wkt, b_wkt)
}

#' Return whether two WKT geometries cross.
#'
#' @export
wbw_topology_crosses_wkt <- function(a_wkt, b_wkt) {
  topology_crosses_wkt(a_wkt, b_wkt)
}

#' Return whether two WKT geometries overlap.
#'
#' @export
wbw_topology_overlaps_wkt <- function(a_wkt, b_wkt) {
  topology_overlaps_wkt(a_wkt, b_wkt)
}

#' Return whether geometry A covers geometry B.
#'
#' @export
wbw_topology_covers_wkt <- function(a_wkt, b_wkt) {
  topology_covers_wkt(a_wkt, b_wkt)
}

#' Return whether geometry A is covered by geometry B.
#'
#' @export
wbw_topology_covered_by_wkt <- function(a_wkt, b_wkt) {
  topology_covered_by_wkt(a_wkt, b_wkt)
}

#' Return DE-9IM relate matrix for two WKT geometries.
#'
#' @export
wbw_topology_relate_wkt <- function(a_wkt, b_wkt) {
  topology_relate_wkt(a_wkt, b_wkt)
}

#' Return planar geometry distance for two WKT geometries.
#'
#' @export
wbw_topology_distance_wkt <- function(a_wkt, b_wkt) {
  topology_distance_wkt(a_wkt, b_wkt)
}

#' Compute topology relation summary between two vector features.
#'
#' `a_vector` and `b_vector` may be `wbw_vector` objects or file paths.
#' Returns a list containing predicate booleans, distance, and DE-9IM matrix.
#'
#' @export
wbw_topology_vector_feature_relation <- function(a_vector, a_feature_index, b_vector, b_feature_index) {
  a_path <- if (inherits(a_vector, "wbw_vector")) {
    a_vector$path
  } else if (is.character(a_vector) && length(a_vector) == 1L && nzchar(a_vector)) {
    a_vector
  } else {
    stop("a_vector must be a wbw_vector object or file path string.", call. = FALSE)
  }

  b_path <- if (inherits(b_vector, "wbw_vector")) {
    b_vector$path
  } else if (is.character(b_vector) && length(b_vector) == 1L && nzchar(b_vector)) {
    b_vector
  } else {
    stop("b_vector must be a wbw_vector object or file path string.", call. = FALSE)
  }

  a_idx <- as.integer(a_feature_index)
  b_idx <- as.integer(b_feature_index)
  if (is.na(a_idx) || a_idx < 0L || is.na(b_idx) || b_idx < 0L) {
    stop("a_feature_index and b_feature_index must be integers >= 0.", call. = FALSE)
  }

  out_json <- topology_vector_feature_relation_json(a_path, a_idx, b_path, b_idx)
  jsonlite::fromJSON(out_json, simplifyVector = TRUE)
}

#' Validate polygon or multipolygon WKT.
#'
#' @export
wbw_topology_is_valid_polygon_wkt <- function(wkt) {
  topology_is_valid_polygon_wkt(wkt)
}

#' Repair polygon or multipolygon WKT.
#'
#' Returns repaired geometry as MULTIPOLYGON WKT.
#'
#' @export
wbw_topology_make_valid_polygon_wkt <- function(wkt, epsilon = 1e-9) {
  topology_make_valid_polygon_wkt(wkt, as.numeric(epsilon))
}

#' Buffer point/linestring/polygon WKT.
#'
#' Returns buffered geometry as POLYGON WKT.
#'
#' @export
wbw_topology_buffer_wkt <- function(wkt, distance) {
  topology_buffer_wkt(wkt, as.numeric(distance))
}

wbw_output_sort_key <- function(key) {
  if (identical(key, "output")) {
    return("0-00000-output")
  }

  if (startsWith(key, "output")) {
    suffix <- substring(key, nchar("output") + 1L)
    if (nzchar(suffix) && grepl("^[0-9]+$", suffix)) {
      return(sprintf("1-%05d-%s", as.integer(suffix), key))
    }
  }

  if (endsWith(key, "_output")) {
    return(sprintf("2-00000-%s", sub("_output$", "", key)))
  }

  sprintf("3-00000-%s", key)
}

wbw_extract_output_path <- function(value) {
  if (is.character(value) && length(value) == 1L && nzchar(value)) {
    return(value)
  }

  if (is.list(value) && !is.null(value$path) && is.character(value$path) && length(value$path) == 1L && nzchar(value$path)) {
    return(value$path)
  }

  NULL
}

wbw_infer_data_object_kind <- function(path, value = NULL) {
  typed_kind <- NULL
  if (is.list(value) && !is.null(value$`__wbw_type__`) && is.character(value$`__wbw_type__`)) {
    typed_kind <- tolower(value$`__wbw_type__`[[1]])
  }
  if (!is.null(typed_kind) && typed_kind %in% c("raster", "vector", "lidar")) {
    return(typed_kind)
  }

  ext <- tolower(tools::file_ext(path))
  if (ext %in% c("tif", "tiff", "dep", "bil", "flt", "sdat", "rdc", "asc")) {
    return("raster")
  }
  if (ext %in% c("shp", "geojson", "gpkg", "json")) {
    return("vector")
  }
  if (ext %in% c("las", "laz", "zlidar")) {
    return("lidar")
  }

  NULL
}

wbw_make_data_object_from_path <- function(kind, path, session = NULL) {
  if (!is.character(path) || length(path) != 1L || !nzchar(path)) {
    return(NULL)
  }

  is_memory_path <- startsWith(path, "memory://")

  normalized <- if (is_memory_path) {
    path
  } else {
    normalizePath(path, winslash = "/", mustWork = FALSE)
  }
  if (!is_memory_path && !file.exists(normalized)) {
    return(NULL)
  }

  switch(
    kind,
    raster = wbw_raster_from_path(normalized, session = session),
    vector = wbw_vector_from_path(normalized, session = session),
    lidar = wbw_lidar_from_path(normalized, session = session),
    NULL
  )
}

wbw_coerce_tool_output <- function(outputs, session = NULL) {
  if (!is.list(outputs) || is.null(names(outputs))) {
    return(outputs)
  }

  candidate_keys <- names(outputs)[!vapply(names(outputs), wbw_output_key_is_metadata, logical(1))]

  if (length(candidate_keys) < 2L) {
    return(outputs)
  }

  candidate_keys <- candidate_keys[order(vapply(candidate_keys, wbw_output_sort_key, character(1)))]
  out <- vector("list", length(candidate_keys))

  for (i in seq_along(candidate_keys)) {
    key <- candidate_keys[[i]]
    value <- outputs[[key]]
    path <- wbw_extract_output_path(value)
    kind <- wbw_infer_data_object_kind(path %||% "", value)
    if (is.null(path) || is.null(kind)) {
      if (grepl("output", key, fixed = TRUE)) {
        return(outputs)
      }
      return(outputs)
    }

    obj <- wbw_make_data_object_from_path(kind, path, session = session)
    if (is.null(obj)) {
      return(outputs)
    }
    out[[i]] <- obj
  }

  out
}

wbw_coerce_progress_result <- function(result, session = NULL) {
  if (is.list(result) && "outputs" %in% names(result)) {
    result$outputs <- wbw_coerce_tool_output(result$outputs, session = session)
  }
  result
}

wbw_progress_result_fallback <- function(tool_id, outputs) {
  list(tool_id = tool_id, outputs = outputs, progress = list())
}

#' Create a standard progress-printing callback.
#'
#' The returned function accepts `(pct, message)` arguments, matching
#' `wbw_run_tool_with_progress(..., on_progress = ...)` callback shape.
#'
#' @param show_messages logical; when `TRUE`, non-progress message text is printed.
#' @param min_increment integer percentage increment required before printing again.
#' @param stream output stream/connection; defaults to `stdout()`.
#'
#' @return A stateful callback function.
#' @export
wbw_make_progress_printer <- function(show_messages = TRUE,
                                      min_increment = 1L,
                                      stream = stdout()) {
  show_messages <- isTRUE(show_messages)
  min_increment <- as.integer(min_increment)
  if (is.na(min_increment) || min_increment < 1L) {
    min_increment <- 1L
  }

  state <- new.env(parent = emptyenv())
  state$last_reported <- -1L

  infer_percent_from_message <- function(msg) {
    if (!nzchar(msg)) {
      return(NA_real_)
    }
    m <- regexpr("(-?[0-9]+(\\.[0-9]+)?)\\s*%", msg, perl = TRUE)
    if (m[[1]] < 0L) {
      return(NA_real_)
    }
    token <- regmatches(msg, m)
    as.numeric(sub("%.*$", "", token))
  }

  emit_percent <- function(value) {
    if (!is.finite(value)) {
      return(invisible(NULL))
    }

    if (value <= 1.0) {
      value <- value * 100.0
    }

    pct_int <- as.integer(base::max(0.0, base::min(100.0, base::floor(value))))

    # If progress decreases, treat as a new run and restart reporting.
    if (pct_int < state$last_reported) {
      state$last_reported <- -1L
    }

    should_print <- (pct_int >= (state$last_reported + min_increment)) || pct_int == 100L
    if (should_print && pct_int > state$last_reported) {
      cat(sprintf("%d%%\n", pct_int), file = stream)
      state$last_reported <- pct_int
    }

    invisible(NULL)
  }

  function(pct = NA_real_, message = "") {
    msg <- if (is.null(message)) "" else as.character(message[[1]])

    if (!is.numeric(pct) || length(pct) == 0L || is.na(pct[[1]])) {
      inferred <- infer_percent_from_message(msg)
      if (!is.na(inferred)) {
        emit_percent(inferred)
      }
      if (show_messages && nzchar(msg)) {
        cat(msg, "\n", sep = "", file = stream)
      }
      return(invisible(NULL))
    }

    value <- as.numeric(pct[[1]])
    if (!is.finite(value)) {
      if (show_messages && nzchar(msg)) {
        cat(msg, "\n", sep = "", file = stream)
      }
      return(invisible(NULL))
    }
    emit_percent(value)
  }
}

.wbw_default_progress_printer <- wbw_make_progress_printer()

#' Built-in standard progress callback.
#'
#' This callback can be passed directly as `on_progress` to
#' `wbw_run_tool_with_progress(...)`.
#'
#' @param pct numeric progress value in `[0, 1]` or `[0, 100]`.
#' @param message optional status message.
#' @export
wbw_print_progress <- function(pct = NA_real_, message = "") {
  .wbw_default_progress_printer(pct, message)
}

wbw_args_to_json <- function(args) {
  if (length(args) == 0L) {
    return("{}")
  }
  jsonlite::toJSON(args, auto_unbox = TRUE, null = "null")
}

wbw_merge_write_options <- function(options = NULL,
                                    compress = NULL,
                                    strict_format_options = FALSE) {
  if (is.null(options)) {
    options <- list()
  }
  if (!is.list(options)) {
    stop("options must be a list or NULL.", call. = FALSE)
  }

  if (!is.null(compress)) {
    if (!is.logical(compress) || length(compress) != 1L || is.na(compress)) {
      stop("compress must be TRUE/FALSE when provided.", call. = FALSE)
    }
    options$compress <- isTRUE(compress)
  }

  if (!is.logical(strict_format_options) || length(strict_format_options) != 1L || is.na(strict_format_options)) {
    stop("strict_format_options must be TRUE/FALSE.", call. = FALSE)
  }
  options$strict_format_options <- isTRUE(strict_format_options)
  options
}

wbw_merge_vector_options <- function(options = NULL,
                                     strict_format_options = FALSE) {
  if (is.null(options)) {
    options <- list()
  }
  if (!is.list(options)) {
    stop("options must be a list or NULL.", call. = FALSE)
  }
  if (!is.logical(strict_format_options) || length(strict_format_options) != 1L || is.na(strict_format_options)) {
    stop("strict_format_options must be TRUE/FALSE.", call. = FALSE)
  }
  options$strict_format_options <- isTRUE(strict_format_options)
  options
}

wbw_path_has_extension <- function(path) {
  ext <- tools::file_ext(path)
  is.character(ext) && length(ext) == 1L && nzchar(ext)
}

wbw_apply_default_output_extension <- function(path, kind = c("raster", "vector", "lidar")) {
  kind <- match.arg(kind)
  if (!is.character(path) || length(path) != 1L || !nzchar(path)) {
    stop("path must be a non-empty string.", call. = FALSE)
  }

  if (wbw_path_has_extension(path)) {
    return(list(path = path, extension_was_missing = FALSE))
  }

  suffix <- switch(
    kind,
    raster = ".tif",
    vector = ".gpkg",
    lidar = ".copc.laz"
  )

  list(path = paste0(path, suffix), extension_was_missing = TRUE)
}

wbw_raster_source_path <- function(raster) {
  if (!inherits(raster, "wbw_raster")) {
    stop("raster must be a wbw_raster object.", call. = FALSE)
  }

  if (!is.null(raster$file_path) && is.function(raster$file_path)) {
    return(raster$file_path())
  }

  if (!is.null(raster$path) && is.character(raster$path) && length(raster$path) == 1L) {
    return(raster$path)
  }

  stop("Unable to resolve source path for raster object.", call. = FALSE)
}

#' Write a raster to disk with optional GeoTIFF/COG controls.
#'
#' @export
wbw_write_raster <- function(raster,
                             output_path,
                             options = NULL,
                             compress = NULL,
                             strict_format_options = FALSE,
                             session = NULL) {
  src <- wbw_raster_source_path(raster)
  opts <- wbw_merge_write_options(
    options = options,
    compress = compress,
    strict_format_options = strict_format_options
  )

  resolved <- wbw_apply_default_output_extension(output_path, kind = "raster")
  output_path_resolved <- resolved$path

  if (isTRUE(resolved$extension_was_missing)) {
    if (is.null(opts$geotiff)) {
      opts$geotiff <- list()
    }
    if (is.null(opts$geotiff$layout)) {
      opts$geotiff$layout <- "cog"
    }
  }

  opts_json <- jsonlite::toJSON(opts, auto_unbox = TRUE, null = "null")

  raster_write_with_options_json(src, output_path_resolved, opts_json)
  wbw_raster_from_path(output_path_resolved, session = session)
}

#' Write multiple rasters to disk with optional GeoTIFF/COG controls.
#'
#' @export
wbw_write_rasters <- function(rasters,
                              output_paths,
                              options = NULL,
                              compress = NULL,
                              strict_format_options = FALSE,
                              session = NULL) {
  if (!is.list(rasters)) {
    stop("rasters must be a list of wbw_raster objects.", call. = FALSE)
  }
  if (!is.character(output_paths)) {
    stop("output_paths must be a character vector.", call. = FALSE)
  }
  if (length(rasters) != length(output_paths)) {
    stop("rasters and output_paths must have the same length.", call. = FALSE)
  }

  out <- vector("list", length(rasters))
  for (i in seq_along(rasters)) {
    out[[i]] <- wbw_write_raster(
      raster = rasters[[i]],
      output_path = output_paths[[i]],
      options = options,
      compress = compress,
      strict_format_options = strict_format_options,
      session = session
    )
  }
  out
}

#' Write a vector to disk with optional format-specific controls.
#'
#' @export
wbw_write_vector <- function(vector,
                             output_path,
                             options = NULL,
                             strict_format_options = FALSE,
                             session = NULL) {
  if (!inherits(vector, "wbw_vector")) {
    stop("vector must be a wbw_vector object.", call. = FALSE)
  }
  src <- vector$path
  resolved <- wbw_apply_default_output_extension(output_path, kind = "vector")
  output_path_resolved <- resolved$path
  opts <- wbw_merge_vector_options(
    options = options,
    strict_format_options = strict_format_options
  )
  opts_json <- jsonlite::toJSON(opts, auto_unbox = TRUE, null = "null")
  written <- vector_copy_with_options_json(src, output_path_resolved, opts_json)
  wbw_vector_from_path(written, session = session)
}

wbw_require_terra <- function(call_name) {
  if (!requireNamespace("terra", quietly = TRUE)) {
    stop(sprintf("%s requires the 'terra' package.", call_name), call. = FALSE)
  }
}

wbw_raster_from_path <- function(path, session = NULL, proxy = FALSE) {
  if (!is.character(path) || length(path) != 1L || !nzchar(path)) {
    stop("path must be a non-empty string.", call. = FALSE)
  }

  is_memory_path <- startsWith(path, "memory://")
  if (!is_memory_path && !file.exists(path)) {
    stop(sprintf("Raster file does not exist: %s", path), call. = FALSE)
  }

  raster_path <- if (is_memory_path) {
    path
  } else {
    normalizePath(path, winslash = "/", mustWork = TRUE)
  }

  .meta_cache <- NULL
  .get_meta <- function() {
    if (is.null(.meta_cache)) {
      raw <- jsonlite::fromJSON(raster_metadata_json(raster_path), simplifyVector = TRUE)
      crs_text <- NULL
      if (!is.null(raw$crs_wkt) && nzchar(raw$crs_wkt)) {
        crs_text <- raw$crs_wkt
      } else if (!is.null(raw$crs_epsg) && !is.na(raw$crs_epsg)) {
        crs_text <- sprintf("EPSG:%d", as.integer(raw$crs_epsg))
      }
      .meta_cache <<- list(
        rows       = raw$rows,
        columns    = raw$cols,
        bands      = raw$bands,
        x_min      = raw$x_min,
        y_min      = raw$y_min,
        x_max      = raw$x_max,
        y_max      = raw$y_max,
        cell_size_x = raw$cell_size_x,
        cell_size_y = raw$cell_size_y,
        nodata     = raw$nodata,
        data_type  = raw$data_type,
        crs        = crs_text,
        crs_wkt    = raw$crs_wkt,
        crs_epsg   = raw$crs_epsg,
        extent     = c(xmin = raw$x_min, xmax = raw$x_max,
                       ymin = raw$y_min, ymax = raw$y_max),
        resolution = c(raw$cell_size_x, raw$cell_size_y),
        path       = raster_path,
        proxy      = isTRUE(proxy)
      )
    }
    .meta_cache
  }

  metadata <- function() .get_meta()

  file_path <- function() raster_path

  band_count <- function() .get_meta()$bands

  active_band <- function() 1L

  crs_epsg <- function(strict = FALSE) {
    epsg <- .get_meta()$crs_epsg
    if (is.null(epsg) || is.na(epsg)) {
      if (isTRUE(strict)) NULL else NULL
    } else {
      as.integer(epsg)
    }
  }

  crs_wkt <- function() {
    meta <- .get_meta()
    wkt <- meta$crs_wkt
    if (!is.null(wkt) && nzchar(wkt)) {
      return(wkt)
    }
    epsg <- meta$crs_epsg
    if (!is.null(epsg) && !is.na(epsg)) {
      return(sprintf("EPSG:%d", as.integer(epsg)))
    }
    NULL
  }

  to_array <- function(simplify_single_band = TRUE) {
    wbw_require_terra("wbw_raster$to_array()")
    wbw_raster_to_array(raster_path, simplify_single_band = simplify_single_band)
  }

  to_stars <- function(proxy = FALSE) {
    wbw_require_terra("wbw_raster$to_stars()")
    wbw_raster_to_stars(raster_path, proxy = proxy)
  }

  .resolve_raster_operand <- function(other) {
    if (inherits(other, "wbw_raster") && is.function(other$file_path)) {
      return(other$file_path())
    }
    if (is.character(other) && length(other) == 1L && nzchar(other)) {
      if (!file.exists(other)) {
        stop(sprintf("Raster operand path does not exist: %s", other), call. = FALSE)
      }
      return(normalizePath(other, winslash = "/", mustWork = TRUE))
    }
    stop("other must be a wbw_raster object or a valid raster file path.", call. = FALSE)
  }

  .run_binary_raster_math <- function(tool_id, other, output_path = NULL) {
    other_path <- .resolve_raster_operand(other)
    if (is.null(output_path)) {
      output_path <- tempfile(pattern = paste0(tool_id, "_"), fileext = ".tif")
    }
    if (!is.character(output_path) || length(output_path) != 1L || !nzchar(output_path)) {
      stop("output_path must be a non-empty string or NULL.", call. = FALSE)
    }

    output_dir <- dirname(output_path)
    if (!dir.exists(output_dir)) {
      dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)
    }

    args <- list(input1 = raster_path, input2 = other_path, output = output_path)
    if (!is.null(session) && is.function(session$run_tool)) {
      session$run_tool(tool_id, args)
    } else {
      wbw_run_tool(tool_id, args)
    }

    wbw_raster_from_path(output_path, session = session)
  }

  .run_unary_raster_math <- function(tool_id, output_path = NULL) {
    if (is.null(output_path)) {
      output_path <- tempfile(pattern = paste0(tool_id, "_"), fileext = ".tif")
    }
    if (!is.character(output_path) || length(output_path) != 1L || !nzchar(output_path)) {
      stop("output_path must be a non-empty string or NULL.", call. = FALSE)
    }

    output_dir <- dirname(output_path)
    if (!dir.exists(output_dir)) {
      dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)
    }

    args <- list(input = raster_path, output = output_path)
    if (!is.null(session) && is.function(session$run_tool)) {
      session$run_tool(tool_id, args)
    } else {
      wbw_run_tool(tool_id, args)
    }

    wbw_raster_from_path(output_path, session = session)
  }

  add <- function(other, output_path = NULL) {
    .run_binary_raster_math("add", other, output_path = output_path)
  }

  subtract <- function(other, output_path = NULL) {
    .run_binary_raster_math("subtract", other, output_path = output_path)
  }

  multiply <- function(other, output_path = NULL) {
    .run_binary_raster_math("multiply", other, output_path = output_path)
  }

  divide <- function(other, output_path = NULL) {
    .run_binary_raster_math("divide", other, output_path = output_path)
  }

  abs <- function(output_path = NULL) {
    .run_unary_raster_math("abs", output_path = output_path)
  }

  ceil <- function(output_path = NULL) {
    .run_unary_raster_math("ceil", output_path = output_path)
  }

  floor <- function(output_path = NULL) {
    .run_unary_raster_math("floor", output_path = output_path)
  }

  round <- function(output_path = NULL) {
    .run_unary_raster_math("round", output_path = output_path)
  }

  square <- function(output_path = NULL) {
    .run_unary_raster_math("square", output_path = output_path)
  }

  sqrt <- function(output_path = NULL) {
    .run_unary_raster_math("sqrt", output_path = output_path)
  }

  log10 <- function(output_path = NULL) {
    .run_unary_raster_math("log10", output_path = output_path)
  }

  log2 <- function(output_path = NULL) {
    .run_unary_raster_math("log2", output_path = output_path)
  }

  sin <- function(output_path = NULL) {
    .run_unary_raster_math("sin", output_path = output_path)
  }

  cos <- function(output_path = NULL) {
    .run_unary_raster_math("cos", output_path = output_path)
  }

  tan <- function(output_path = NULL) {
    .run_unary_raster_math("tan", output_path = output_path)
  }

  sinh <- function(output_path = NULL) {
    .run_unary_raster_math("sinh", output_path = output_path)
  }

  cosh <- function(output_path = NULL) {
    .run_unary_raster_math("cosh", output_path = output_path)
  }

  tanh <- function(output_path = NULL) {
    .run_unary_raster_math("tanh", output_path = output_path)
  }

  exp <- function(output_path = NULL) {
    .run_unary_raster_math("exp", output_path = output_path)
  }

  exp2 <- function(output_path = NULL) {
    .run_unary_raster_math("exp2", output_path = output_path)
  }

  deep_copy <- function(output_path, overwrite = FALSE) {
    if (!is.character(output_path) || length(output_path) != 1L || !nzchar(output_path)) {
      stop("output_path must be a non-empty string.", call. = FALSE)
    }
    output_dir <- dirname(output_path)
    if (!dir.exists(output_dir)) {
      dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)
    }
    copied <- file.copy(raster_path, output_path, overwrite = overwrite)
    if (!isTRUE(copied)) {
      stop(sprintf("Failed to copy raster to %s", output_path), call. = FALSE)
    }
    wbw_raster_from_path(output_path, session = session)
  }

  write <- function(output_path,
                    overwrite = FALSE,
                    options = NULL,
                    compress = NULL,
                    strict_format_options = FALSE) {
    if (isTRUE(overwrite) && file.exists(output_path)) {
      unlink(output_path)
    }
    wbw_write_raster(
      raster = obj,
      output_path = output_path,
      options = options,
      compress = compress,
      strict_format_options = strict_format_options,
      session = session
    )
  }

  obj <- new.env(parent = emptyenv())
  obj$path <- raster_path
  obj$session <- session
  obj$metadata <- metadata
  obj$file_path <- file_path
  obj$band_count <- band_count
  obj$active_band <- active_band
  obj$crs_epsg <- crs_epsg
  obj$crs_wkt <- crs_wkt
  obj$to_array <- to_array
  obj$to_stars <- to_stars
  obj$add <- add
  obj$subtract <- subtract
  obj$multiply <- multiply
  obj$divide <- divide
  obj$abs <- abs
  obj$ceil <- ceil
  obj$floor <- floor
  obj$round <- round
  obj$square <- square
  obj$sqrt <- sqrt
  obj$log10 <- log10
  obj$log2 <- log2
  obj$sin <- sin
  obj$cos <- cos
  obj$tan <- tan
  obj$sinh <- sinh
  obj$cosh <- cosh
  obj$tanh <- tanh
  obj$exp <- exp
  obj$exp2 <- exp2
  obj$deep_copy <- deep_copy
  obj$write <- write
  class(obj) <- c("wbw_raster", "wbw_data_object")
  obj
}

#' @export
"+.wbw_raster" <- function(e1, e2) {
  e1$add(e2)
}

#' @export
"-.wbw_raster" <- function(e1, e2) {
  if (missing(e2)) {
    stop("Unary negation is not yet implemented for wbw_raster. Use r$multiply(constant_raster) for cell-wise scaling.", call. = FALSE)
  }
  e1$subtract(e2)
}

#' @export
"*.wbw_raster" <- function(e1, e2) {
  e1$multiply(e2)
}

#' @export
"/.wbw_raster" <- function(e1, e2) {
  e1$divide(e2)
}

wbw_vector_from_path <- function(path,
                                 session = NULL,
                                 read_options = NULL,
                                 strict_format_options = FALSE) {
  if (!file.exists(path)) {
    stop(sprintf("Vector file does not exist: %s", path), call. = FALSE)
  }

  vector_path <- normalizePath(path, winslash = "/", mustWork = TRUE)

  if (!is.null(read_options)) {
    opts <- wbw_merge_vector_options(
      options = read_options,
      strict_format_options = strict_format_options
    )
    opts_json <- jsonlite::toJSON(opts, auto_unbox = TRUE, null = "null")
    tmp_out <- tempfile(pattern = "wbw_vector_read_", fileext = ".gpkg")
    vector_path <- vector_copy_with_options_json(vector_path, tmp_out, opts_json)
  }

  .meta_cache <- NULL
  .get_meta <- function() {
    if (is.null(.meta_cache)) {
      raw <- jsonlite::fromJSON(vector_metadata_json(vector_path), simplifyVector = FALSE)
      geom <- tolower(raw$geometry_type %||% "unknown")
      geom <- switch(
        geom,
        "polygon" = "polygons",
        "multipolygon" = "polygons",
        "linestring" = "lines",
        "multilinestring" = "lines",
        "point" = "points",
        "multipoint" = "points",
        geom
      )
      fields_raw <- raw$fields %||% list()
      field_names <- vapply(fields_raw, function(f) f$name, character(1))
      field_types <- vapply(fields_raw, function(f) f$field_type, character(1))
      .meta_cache <<- list(
        geometry_type = geom,
        feature_count = raw$feature_count %||% 0L,
        crs_wkt       = raw$crs_wkt,
        crs_epsg      = raw$crs_epsg,
        fields        = field_names,
        field_types   = field_types,
        path          = vector_path
      )
    }
    .meta_cache
  }

  metadata <- function() .get_meta()

  schema <- function() {
    meta <- metadata()
    data.frame(
      name = meta$fields,
      type = meta$field_types,
      row.names = NULL,
      stringsAsFactors = FALSE
    )
  }

  attributes <- function(feature_index) {
    if (!is.numeric(feature_index) || length(feature_index) != 1L || feature_index < 1) {
      stop(
        "feature_index must be a positive integer.",
        call. = FALSE
      )
    }
    tidx <- as.integer(feature_index)
    terra_vec <- to_terra()
    if (tidx > terra::nrow(terra_vec)) {
      stop(
        sprintf("feature_index %d exceeds feature count %d", tidx, terra::nrow(terra_vec)),
        call. = FALSE
      )
    }
    values_df <- terra::values(terra_vec[tidx, ], as.data.frame = TRUE)
    as.list(values_df[1, , drop = FALSE])
  }

  attribute <- function(feature_index, field_name) {
    if (!is.character(field_name) || length(field_name) != 1L) {
      stop("field_name must be a single string.", call. = FALSE)
    }
    attrs <- attributes(feature_index)
    if (!(field_name %in% names(attrs))) {
      stop(
        sprintf("field '%s' not found in feature schema", field_name),
        call. = FALSE
      )
    }
    attrs[[field_name]]
  }

  update_attributes <- function(feature_index, values_dict) {
    if (!is.list(values_dict)) {
      stop("values_dict must be a named list.", call. = FALSE)
    }
    wbw_require_terra("wbw_vector$update_attributes()")
    terra_vec <- to_terra()
    tidx <- as.integer(feature_index)
    if (tidx > terra::nrow(terra_vec)) {
      stop(
        sprintf("feature_index %d exceeds feature count %d", tidx, terra::nrow(terra_vec)),
        call. = FALSE
      )
    }

    # Update fields via terra
    for (field_name in names(values_dict)) {
      terra_vec[[field_name]][tidx] <- values_dict[[field_name]]
    }

    # Write back to file
    deep_copy(vector_path, overwrite = TRUE)
    invisible(NULL)
  }

  update_attribute <- function(feature_index, field_name, value) {
    if (!is.character(field_name) || length(field_name) != 1L) {
      stop("field_name must be a single string.", call. = FALSE)
    }
    update_attributes(feature_index, setNames(list(value), field_name))
  }

  add_field <- function(field_name, field_type = "text", default_value = NA) {
    if (!is.character(field_name) || length(field_name) != 1L) {
      stop("field_name must be a single string.", call. = FALSE)
    }
    valid_types <- c("integer", "float", "text", "date", "datetime", "boolean", "blob", "json")
    if (!(field_type %in% valid_types)) {
      stop(
        sprintf(
          "field_type must be one of: %s, got '%s'",
          paste(valid_types, collapse = ", "),
          field_type
        ),
        call. = FALSE
      )
    }

    wbw_require_terra("wbw_vector$add_field()")
    terra_vec <- to_terra()

    # Map wbw field type to R type for terra
    r_type <- switch(field_type,
      "integer" = 0L,
      "float" = 0.0,
      "text" = "",
      "date" = NA_character_,
      "datetime" = NA_character_,
      "boolean" = NA,
      "blob" = NA_character_,
      "json" = NA_character_,
      NA
    )

    # Add field via terra with default value
    if (is.na(default_value)) {
      terra_vec[[field_name]] <- r_type
    } else {
      terra_vec[[field_name]] <- default_value
    }

    # Write back to file
    deep_copy(vector_path, overwrite = TRUE)
    invisible(NULL)
  }

  to_terra <- function() {
    wbw_require_terra("wbw_vector$to_terra()")
    terra::vect(vector_path)
  }

  to_sf <- function() {
    if (!requireNamespace("sf", quietly = TRUE)) {
      stop("wbw_vector$to_sf() requires the 'sf' package.", call. = FALSE)
    }
    sf::st_read(vector_path, quiet = TRUE)
  }

  deep_copy <- function(output_path,
                        overwrite = FALSE,
                        options = NULL,
                        strict_format_options = FALSE) {
    if (!is.character(output_path) || length(output_path) != 1L || !nzchar(output_path)) {
      stop("output_path must be a non-empty string.", call. = FALSE)
    }
    if (isTRUE(overwrite) && file.exists(output_path)) {
      unlink(output_path)
    }
    wbw_write_vector(
      vector = obj,
      output_path = output_path,
      options = options,
      strict_format_options = strict_format_options,
      session = session
    )
  }

  write <- function(output_path,
                    overwrite = FALSE,
                    options = NULL,
                    strict_format_options = FALSE) {
    deep_copy(
      output_path,
      overwrite = overwrite,
      options = options,
      strict_format_options = strict_format_options
    )
  }

  obj <- new.env(parent = emptyenv())
  obj$path <- vector_path
  obj$session <- session
  obj$metadata <- metadata
  obj$schema <- schema
  obj$attributes <- attributes
  obj$attribute <- attribute
  obj$update_attributes <- update_attributes
  obj$update_attribute <- update_attribute
  obj$add_field <- add_field
  obj$to_terra <- to_terra
  obj$to_sf <- to_sf
  obj$deep_copy <- deep_copy
  obj$write <- write
  class(obj) <- c("wbw_vector", "wbw_data_object")
  obj
}

wbw_lidar_metadata <- function(path) {
  jsonlite::fromJSON(lidar_metadata_json(path), simplifyVector = FALSE)
}

wbw_sensor_bundle_metadata <- function(path) {
  jsonlite::fromJSON(sensor_bundle_metadata_json(path), simplifyVector = FALSE)
}

wbw_sensor_bundle_read_key_as_raster <- function(bundle_root, key, key_type, session = NULL) {
  if (!is.character(key) || length(key) != 1L || !nzchar(key)) {
    stop("key must be a non-empty string.", call. = FALSE)
  }
  path <- sensor_bundle_resolve_raster_path(bundle_root, key, key_type)
  wbw_read_raster(path, session = session)
}

wbw_normalize_bundle_key <- function(key) {
  gsub("[^a-z0-9]+", "", tolower(key))
}

wbw_match_bundle_key <- function(available_keys, candidates) {
  if (length(available_keys) == 0L || length(candidates) == 0L) {
    return(NULL)
  }

  extract_band_number <- function(key) {
    normalized <- gsub("[^a-z0-9]+", "_", tolower(key))
    m <- regmatches(normalized, regexec("(^|_)b0*([0-9]{1,2})($|_)", normalized, perl = TRUE))[[1]]
    if (length(m) >= 3L) {
      return(as.integer(m[[3]]))
    }
    NA_integer_
  }

  for (candidate in candidates) {
    if (candidate %in% available_keys) {
      return(candidate)
    }
  }

  normalized_available <- vapply(available_keys, wbw_normalize_bundle_key, character(1))
  for (candidate in candidates) {
    normalized_candidate <- wbw_normalize_bundle_key(candidate)
    idx <- which(normalized_available == normalized_candidate)
    if (length(idx) > 0L) {
      return(available_keys[[idx[[1]]]])
    }

    # Allow common band aliases like B4 <-> B04 and SR_B4 <-> SR_B04.
    candidate_band <- extract_band_number(candidate)
    if (!is.na(candidate_band)) {
      available_bands <- vapply(available_keys, extract_band_number, integer(1))
      idx <- which(!is.na(available_bands) & available_bands == candidate_band)
      if (length(idx) > 0L) {
        return(available_keys[[idx[[1]]]])
      }
    }

    # Handle prefixed SAR keys like IW1_SLC_VV by token containment.
    if (nchar(normalized_candidate) >= 2L) {
      idx <- which(grepl(normalized_candidate, normalized_available, fixed = TRUE))
      if (length(idx) > 0L) {
        return(available_keys[[idx[[1]]]])
      }
    }
  }

  NULL
}

wbw_bundle_key_list_by_type <- function(bundle, key_type) {
  switch(
    key_type,
    band = bundle$list_band_keys(),
    measurement = bundle$list_measurement_keys(),
    qa = bundle$list_qa_keys(),
    aux = bundle$list_aux_keys(),
    asset = bundle$list_asset_keys(),
    stop(sprintf("Unsupported key_type: %s", key_type), call. = FALSE)
  )
}

wbw_bundle_read_by_type <- function(bundle, key, key_type) {
  switch(
    key_type,
    band = bundle$read_band(key),
    measurement = bundle$read_measurement(key),
    qa = bundle$read_qa_layer(key),
    aux = bundle$read_aux_layer(key),
    asset = bundle$read_asset(key),
    stop(sprintf("Unsupported key_type: %s", key_type), call. = FALSE)
  )
}

wbw_bundle_pick_preview_raster <- function(bundle) {
  family <- tolower(bundle$family %||% "")
  if (family %in% c("sentinel1_safe", "radarsat2", "rcm")) {
    key_types <- c("measurement", "asset", "band", "qa", "aux")
  } else if (identical(family, "iceye")) {
    key_types <- c("asset", "measurement", "band", "qa", "aux")
  } else {
    key_types <- c("band", "measurement", "asset", "qa", "aux")
  }

  for (key_type in key_types) {
    keys <- wbw_bundle_key_list_by_type(bundle, key_type)
    if (length(keys) == 0L) {
      next
    }
    key <- keys[[1]]
    return(list(kind = key_type, key = key, raster = wbw_bundle_read_by_type(bundle, key, key_type)))
  }
  NULL
}

wbw_bundle_enhance_candidates <- function(family, base_candidates, bundle) {
  if (is.null(bundle)) {
    return(base_candidates)
  }

  available_keys <- c(
    wbw_bundle_key_list_by_type(bundle, "band"),
    wbw_bundle_key_list_by_type(bundle, "measurement"),
    wbw_bundle_key_list_by_type(bundle, "asset")
  )

  if (length(available_keys) == 0L) {
    return(base_candidates)
  }

  available_upper <- toupper(available_keys)
  available_lower <- tolower(available_keys)

  enhance <- function(candidates) {
    if (is.null(candidates) || length(candidates) == 0L) {
      return(candidates)
    }

    for (candidate in candidates) {
      candidate_upper <- toupper(candidate)
      if (candidate_upper %in% available_upper) {
        return(candidates)
      }
      if (candidate %in% available_keys) {
        return(candidates)
      }
      if (candidate %in% available_lower) {
        return(candidates)
      }
    }

    unique(c(candidates, available_keys[seq_len(min(5, length(available_keys)))]))
  }

  list(
    red = enhance(base_candidates$red),
    green = enhance(base_candidates$green),
    blue = enhance(base_candidates$blue)
  )
}

wbw_bundle_style_candidates <- function(family, style, bundle = NULL) {
  family <- tolower(family %||% "")

  base_cand <- NULL
  if (identical(style, "true_colour")) {
    if (identical(family, "landsat")) {
      base_cand <- list(
        red = c("B4", "SR_B4", "red", "R"),
        green = c("B3", "SR_B3", "green", "G"),
        blue = c("B2", "SR_B2", "blue", "B")
      )
    } else if (identical(family, "sentinel2_safe")) {
      base_cand <- list(
        red = c("B04", "B4", "red", "R"),
        green = c("B03", "B3", "green", "G"),
        blue = c("B02", "B2", "blue", "B")
      )
    } else if (family %in% c("sentinel1_safe", "radarsat2", "rcm")) {
      base_cand <- list(
        red = c("VV", "HH", "C11", "I"),
        green = c("VH", "HV", "C22", "Q"),
        blue = c("VV", "HH", "C11", "I")
      )
    } else if (identical(family, "iceye")) {
      base_cand <- list(
        red = c("GRD", "VV", "HH", "image", "intensity", "amplitude"),
        green = c("GRD", "VV", "HH", "image", "intensity", "amplitude"),
        blue = c("GRD", "VV", "HH", "image", "intensity", "amplitude")
      )
    } else {
      base_cand <- list(
        red = c("red", "R", "B4", "B04", "SR_B4"),
        green = c("green", "G", "B3", "B03", "SR_B3"),
        blue = c("blue", "B", "B2", "B02", "SR_B2")
      )
    }
  } else {
    if (identical(family, "landsat")) {
      base_cand <- list(
        red = c("B5", "SR_B5", "nir", "NIR", "NIR08"),
        green = c("B4", "SR_B4", "red", "R"),
        blue = c("B3", "SR_B3", "green", "G")
      )
    } else if (identical(family, "sentinel2_safe")) {
      base_cand <- list(
        red = c("B08", "B8", "nir", "NIR", "NIR08"),
        green = c("B04", "B4", "red", "R"),
        blue = c("B03", "B3", "green", "G")
      )
    } else if (family %in% c("sentinel1_safe", "radarsat2", "rcm")) {
      base_cand <- list(
        red = c("VV", "HH", "C11", "I"),
        green = c("VH", "HV", "C22", "Q"),
        blue = c("VH", "HV", "C22", "Q")
      )
    } else if (identical(family, "iceye")) {
      base_cand <- list(
        red = c("GRD", "VV", "HH", "image", "intensity", "amplitude"),
        green = c("GRD", "VV", "HH", "image", "intensity", "amplitude"),
        blue = c("GRD", "VV", "HH", "image", "intensity", "amplitude")
      )
    } else {
      base_cand <- list(
        red = c("nir", "NIR", "NIR08", "B08", "B8", "B5", "SR_B5"),
        green = c("red", "R", "B4", "B04", "SR_B4"),
        blue = c("green", "G", "B3", "B03", "SR_B3")
      )
    }
  }

  return(wbw_bundle_enhance_candidates(family, base_cand, bundle))
}

wbw_bundle_pick_channel_key <- function(bundle, candidates, key_types = c("band", "measurement", "asset")) {
  for (key_type in key_types) {
    keys <- wbw_bundle_key_list_by_type(bundle, key_type)
    if (length(keys) == 0L) {
      next
    }
    matched <- wbw_match_bundle_key(keys, candidates)
    if (!is.null(matched)) {
      return(list(key = matched, key_type = key_type))
    }
  }
  NULL
}

wbw_bundle_resolve_colour_channel <- function(bundle, explicit_key, default_candidates, key_types) {
  if (!is.null(explicit_key)) {
    explicit <- wbw_bundle_pick_channel_key(bundle, c(explicit_key), key_types = key_types)
    if (is.null(explicit)) {
      stop(
        sprintf("Unable to locate explicit bundle key '%s' in key types: %s", explicit_key, paste(key_types, collapse = ", ")),
        call. = FALSE
      )
    }
    return(explicit)
  }

  chosen <- wbw_bundle_pick_channel_key(bundle, default_candidates, key_types = key_types)
  if (is.null(chosen)) {
    stop(
      sprintf("Unable to locate any of the expected keys [%s] in key types: %s", paste(default_candidates, collapse = ", "), paste(key_types, collapse = ", ")),
      call. = FALSE
    )
  }
  chosen
}

wbw_bundle_write_colour_composite <- function(bundle,
                                              style,
                                              output_path = NULL,
                                              red_key = NULL,
                                              green_key = NULL,
                                              blue_key = NULL,
                                              key_types = c("band", "measurement", "asset"),
                                              enhance = TRUE,
                                              treat_zeros_as_nodata = FALSE,
                                              session = NULL) {
  style <- match.arg(style, c("true_colour", "false_colour"))
  if (is.null(output_path)) {
    output_path <- tempfile(pattern = paste0(style, "_"), fileext = ".tif")
  }
  if (!is.character(output_path) || length(output_path) != 1L || !nzchar(output_path)) {
    stop("output_path must be a non-empty string or NULL.", call. = FALSE)
  }

  output_dir <- dirname(output_path)
  if (!dir.exists(output_dir)) {
    dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)
  }

  candidates <- wbw_bundle_style_candidates(bundle$family %||% NULL, style, bundle = bundle)
  red <- wbw_bundle_resolve_colour_channel(bundle, red_key, candidates$red, key_types = key_types)
  green <- wbw_bundle_resolve_colour_channel(bundle, green_key, candidates$green, key_types = key_types)
  blue <- wbw_bundle_resolve_colour_channel(bundle, blue_key, candidates$blue, key_types = key_types)

  red_path <- sensor_bundle_resolve_raster_path(bundle$bundle_root, red$key, red$key_type)
  green_path <- sensor_bundle_resolve_raster_path(bundle$bundle_root, green$key, green$key_type)
  blue_path <- sensor_bundle_resolve_raster_path(bundle$bundle_root, blue$key, blue$key_type)

  active_session <- session %||% bundle$session
  args <- list(
    red = red_path,
    green = green_path,
    blue = blue_path,
    output = output_path,
    enhance = enhance,
    treat_zeros_as_nodata = treat_zeros_as_nodata
  )

  run_err <- NULL
  run_create_colour_composite <- function(tool_args) {
    if (!is.null(active_session) && is.function(active_session$run_tool)) {
      return(active_session$run_tool("create_colour_composite", tool_args))
    }
    wbw_run_tool("create_colour_composite", args = tool_args)
  }

  tryCatch(
    run_create_colour_composite(args),
    error = function(e) {
      run_err <<- e
      NULL
    }
  )

  if (!is.null(run_err)) {
    args$input <- red_path
    run_create_colour_composite(args)
  }

  wbw_read_raster(output_path, session = active_session)
}

wbw_lidar_from_path <- function(path, session = NULL) {
  if (!file.exists(path)) {
    stop(sprintf("Lidar file does not exist: %s", path), call. = FALSE)
  }

  lidar_path <- normalizePath(path, winslash = "/", mustWork = TRUE)

  metadata <- function() {
    wbw_lidar_metadata(lidar_path)
  }

  point_count <- function() {
    as.numeric(lidar_point_count(lidar_path))
  }

  normalize_lidar_fields <- function(fields) {
    if (is.null(fields)) {
      return(c("x", "y", "z"))
    }
    if (!is.character(fields) || length(fields) < 1L) {
      stop("fields must be a non-empty character vector.", call. = FALSE)
    }
    fields <- trimws(fields)
    if (any(!nzchar(fields))) {
      stop("fields must not contain empty names.", call. = FALSE)
    }
    fields
  }

  to_matrix <- function(fields = c("x", "y", "z")) {
    fields <- normalize_lidar_fields(fields)
    payload <- jsonlite::fromJSON(
      lidar_columns_json(
        lidar_path,
        jsonlite::toJSON(fields, auto_unbox = FALSE, null = "null")
      ),
      simplifyVector = FALSE
    )

    columns <- lapply(payload$columns %||% list(), as.numeric)
    if (length(columns) == 0L) {
      return(matrix(numeric(0), nrow = 0L, ncol = 0L))
    }

    mat <- do.call(cbind, columns)
    storage.mode(mat) <- "double"
    colnames(mat) <- payload$fields %||% fields
    mat
  }

  to_data_frame <- function(fields = c("x", "y", "z")) {
    mat <- to_matrix(fields = fields)
    as.data.frame(mat, stringsAsFactors = FALSE)
  }

  to_matrix_chunks <- function(chunk_size,
                               fields = c("x", "y", "z")) {
    if (!is.numeric(chunk_size) || length(chunk_size) != 1L || is.na(chunk_size) || chunk_size < 1) {
      stop("chunk_size must be a positive number.", call. = FALSE)
    }
    chunk_size <- as.integer(chunk_size)
    fields <- normalize_lidar_fields(fields)

    mat <- to_matrix(fields = fields)
    n <- nrow(mat)
    if (n == 0L) {
      return(list())
    }

    starts <- seq.int(1L, n, by = chunk_size)
    chunks <- lapply(starts, function(start_idx) {
      end_idx <- min(start_idx + chunk_size - 1L, n)
      mat[start_idx:end_idx, , drop = FALSE]
    })
    chunks
  }

  from_matrix <- function(matrix_data,
                          output_path,
                          overwrite = FALSE,
                          fields = c("x", "y", "z")) {
    if (is.null(output_path) || !is.character(output_path) || length(output_path) != 1L || !nzchar(output_path)) {
      stop("output_path must be a non-empty string.", call. = FALSE)
    }
    fields <- normalize_lidar_fields(fields)

    if (is.vector(matrix_data) && !is.matrix(matrix_data)) {
      matrix_data <- matrix(matrix_data, ncol = length(fields))
    }
    if (!is.matrix(matrix_data)) {
      stop("matrix_data must be a matrix or numeric vector.", call. = FALSE)
    }
    if (!is.numeric(matrix_data)) {
      stop("matrix_data must be numeric.", call. = FALSE)
    }
    if (ncol(matrix_data) != length(fields)) {
      stop(sprintf("matrix_data has %d columns but fields has %d entries.", ncol(matrix_data), length(fields)), call. = FALSE)
    }

    resolved <- wbw_apply_default_output_extension(output_path, kind = "lidar")
    resolved_path <- resolved$path

    output_dir <- dirname(resolved_path)
    if (!dir.exists(output_dir)) {
      dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)
    }
    if (file.exists(resolved_path) && !isTRUE(overwrite)) {
      stop(sprintf("Failed to write lidar to %s (exists; set overwrite=TRUE)", resolved_path), call. = FALSE)
    }
    if (file.exists(resolved_path) && isTRUE(overwrite)) {
      unlink(resolved_path)
    }

    columns <- lapply(seq_len(ncol(matrix_data)), function(i) as.numeric(matrix_data[, i]))

    out_path <- lidar_from_columns_json(
      lidar_path,
      resolved_path,
      jsonlite::toJSON(fields, auto_unbox = FALSE, null = "null"),
      jsonlite::toJSON(columns, auto_unbox = TRUE, null = "null")
    )
    wbw_lidar_from_path(out_path, session = session)
  }

  from_data_frame <- function(data,
                              output_path,
                              overwrite = FALSE,
                              fields = NULL) {
    if (!is.data.frame(data)) {
      stop("data must be a data.frame.", call. = FALSE)
    }
    inferred <- fields
    if (is.null(inferred)) {
      inferred <- names(data)
    }
    if (is.null(inferred) || length(inferred) < 1L) {
      stop("fields must be provided when data has no column names.", call. = FALSE)
    }
    matrix_data <- as.matrix(data[, inferred, drop = FALSE])
    storage.mode(matrix_data) <- "double"
    from_matrix(matrix_data, output_path = output_path, overwrite = overwrite, fields = inferred)
  }

  from_matrix_chunks <- function(chunks,
                                 output_path,
                                 overwrite = FALSE,
                                 fields = c("x", "y", "z")) {
    fields <- normalize_lidar_fields(fields)
    if (!is.list(chunks) || length(chunks) < 1L) {
      stop("chunks must be a non-empty list of matrices.", call. = FALSE)
    }

    normalized <- lapply(seq_along(chunks), function(i) {
      chunk <- chunks[[i]]
      if (is.vector(chunk) && !is.matrix(chunk)) {
        chunk <- matrix(chunk, ncol = length(fields))
      }
      if (!is.matrix(chunk) || !is.numeric(chunk)) {
        stop(sprintf("chunk %d must be a numeric matrix.", i), call. = FALSE)
      }
      if (ncol(chunk) != length(fields)) {
        stop(
          sprintf(
            "chunk %d has %d columns but fields has %d entries.",
            i,
            ncol(chunk),
            length(fields)
          ),
          call. = FALSE
        )
      }
      storage.mode(chunk) <- "double"
      chunk
    })

    resolved <- wbw_apply_default_output_extension(output_path, kind = "lidar")
    resolved_path <- resolved$path

    output_dir <- dirname(resolved_path)
    if (!dir.exists(output_dir)) {
      dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)
    }
    if (file.exists(resolved_path) && !isTRUE(overwrite)) {
      stop(sprintf("Failed to write lidar to %s (exists; set overwrite=TRUE)", resolved_path), call. = FALSE)
    }
    if (file.exists(resolved_path) && isTRUE(overwrite)) {
      unlink(resolved_path)
    }

    ext <- tolower(tools::file_ext(resolved_path))
    # Streaming chunk rewrite is currently available for LAS/LAZ outputs.
    if (identical(ext, "las") || identical(ext, "laz")) {
      chunk_columns <- lapply(normalized, function(chunk) {
        lapply(seq_len(ncol(chunk)), function(i) as.numeric(chunk[, i]))
      })

      out_path <- lidar_from_column_chunks_json(
        lidar_path,
        resolved_path,
        jsonlite::toJSON(fields, auto_unbox = FALSE, null = "null"),
        jsonlite::toJSON(chunk_columns, auto_unbox = TRUE, null = "null")
      )
      return(wbw_lidar_from_path(out_path, session = session))
    }

    mat <- do.call(rbind, normalized)
    from_matrix(
      mat,
      output_path = resolved_path,
      overwrite = TRUE,
      fields = fields
    )
  }

  get_short_filename <- function() {
    basename(lidar_path)
  }

  deep_copy <- function(output_path, overwrite = FALSE, options = NULL) {
    if (!is.character(output_path) || length(output_path) != 1L || !nzchar(output_path)) {
      stop("output_path must be a non-empty string.", call. = FALSE)
    }
    if (!is.null(options) && !is.list(options)) {
      stop("options must be a list or NULL.", call. = FALSE)
    }

    resolved <- wbw_apply_default_output_extension(output_path, kind = "lidar")
    resolved_path <- resolved$path

    output_dir <- dirname(resolved_path)
    if (!dir.exists(output_dir)) {
      dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)
    }

    if (file.exists(resolved_path) && !isTRUE(overwrite)) {
      stop(sprintf("Failed to copy lidar to %s (exists; set overwrite=TRUE)", resolved_path), call. = FALSE)
    }
    if (file.exists(resolved_path) && isTRUE(overwrite)) {
      unlink(resolved_path)
    }

    options_json <- if (is.null(options) || length(options) == 0L) {
      "{}"
    } else {
      jsonlite::toJSON(options, auto_unbox = TRUE, null = "null")
    }

    copied_path <- lidar_write_with_options_json(lidar_path, resolved_path, options_json)
    wbw_lidar_from_path(copied_path, session = session)
  }

  write <- function(output_path, overwrite = FALSE, options = NULL) {
    deep_copy(output_path, overwrite = overwrite, options = options)
  }

  obj <- new.env(parent = emptyenv())
  obj$path <- lidar_path
  obj$file_path <- lidar_path
  obj$session <- session
  obj$metadata <- metadata
  obj$point_count <- point_count
  obj$get_short_filename <- get_short_filename
  obj$to_matrix <- to_matrix
  obj$to_data_frame <- to_data_frame
  obj$to_matrix_chunks <- to_matrix_chunks
  obj$from_matrix <- from_matrix
  obj$from_data_frame <- from_data_frame
  obj$from_matrix_chunks <- from_matrix_chunks
  obj$deep_copy <- deep_copy
  obj$write <- write
  class(obj) <- c("wbw_lidar", "wbw_data_object")
  obj
}

wbw_bundle_expect_family <- function(bundle, expected_family) {
  actual_family <- bundle$metadata()$family %||% ""
  if (!identical(actual_family, expected_family)) {
    stop(
      sprintf("Expected %s bundle, detected %s", expected_family, actual_family),
      call. = FALSE
    )
  }
  bundle
}

wbw_sensor_bundle_from_path <- function(path, session = NULL) {
  if (!file.exists(path)) {
    stop(sprintf("Sensor bundle path does not exist: %s", path), call. = FALSE)
  }

  input_path <- normalizePath(path, winslash = "/", mustWork = TRUE)

  metadata <- function() {
    wbw_sensor_bundle_metadata(input_path)
  }

  list_band_keys <- function() {
    metadata()$band_keys %||% list()
  }

  list_measurement_keys <- function() {
    metadata()$measurement_keys %||% list()
  }

  list_qa_keys <- function() {
    metadata()$qa_keys %||% list()
  }

  list_aux_keys <- function() {
    metadata()$aux_keys %||% list()
  }

  list_asset_keys <- function() {
    metadata()$asset_keys %||% list()
  }

  key_summary <- function() {
    counts <- c(
      band = length(list_band_keys()),
      measurement = length(list_measurement_keys()),
      qa = length(list_qa_keys()),
      aux = length(list_aux_keys()),
      asset = length(list_asset_keys())
    )

    data.frame(
      key_type = names(counts),
      key_count = as.integer(counts),
      has_any = as.logical(counts > 0L),
      row.names = NULL,
      stringsAsFactors = FALSE
    )
  }

  resolve_key <- function(key, key_types = c("band", "measurement", "qa", "aux", "asset")) {
    if (!is.character(key) || length(key) != 1L || !nzchar(key)) {
      stop("key must be a non-empty string.", call. = FALSE)
    }
    key_types <- as.character(key_types)
    if (length(key_types) == 0L) {
      stop("key_types must contain at least one key type.", call. = FALSE)
    }

    resolved <- wbw_bundle_pick_channel_key(obj, c(key), key_types = key_types)
    if (is.null(resolved)) {
      stop(
        sprintf("Unable to locate key '%s' in key types: %s", key, paste(key_types, collapse = ", ")),
        call. = FALSE
      )
    }
    resolved
  }

  has_key <- function(key, key_types = c("band", "measurement", "qa", "aux", "asset")) {
    !is.null(wbw_bundle_pick_channel_key(obj, c(key), key_types = as.character(key_types)))
  }

  read_any <- function(key, key_types = c("band", "measurement", "qa", "aux", "asset")) {
    resolved <- resolve_key(key, key_types = key_types)
    wbw_bundle_read_by_type(obj, resolved$key, resolved$key_type)
  }

  read_band <- function(key) {
    wbw_sensor_bundle_read_key_as_raster(obj$bundle_root, key, "band", session = session)
  }

  read_measurement <- function(key) {
    wbw_sensor_bundle_read_key_as_raster(obj$bundle_root, key, "measurement", session = session)
  }

  read_qa_layer <- function(key) {
    wbw_sensor_bundle_read_key_as_raster(obj$bundle_root, key, "qa", session = session)
  }

  read_aux_layer <- function(key) {
    wbw_sensor_bundle_read_key_as_raster(obj$bundle_root, key, "aux", session = session)
  }

  read_asset <- function(key) {
    wbw_sensor_bundle_read_key_as_raster(obj$bundle_root, key, "asset", session = session)
  }

  read_preview_raster <- function() {
    wbw_bundle_pick_preview_raster(obj)
  }

  write_true_colour <- function(output_path = NULL,
                                red_key = NULL,
                                green_key = NULL,
                                blue_key = NULL,
                                key_types = c("band", "measurement", "asset"),
                                enhance = TRUE,
                                treat_zeros_as_nodata = FALSE) {
    wbw_bundle_write_colour_composite(
      obj,
      style = "true_colour",
      output_path = output_path,
      red_key = red_key,
      green_key = green_key,
      blue_key = blue_key,
      key_types = key_types,
      enhance = enhance,
      treat_zeros_as_nodata = treat_zeros_as_nodata,
      session = session
    )
  }

  write_false_colour <- function(output_path = NULL,
                                 red_key = NULL,
                                 green_key = NULL,
                                 blue_key = NULL,
                                 key_types = c("band", "measurement", "asset"),
                                 enhance = TRUE,
                                 treat_zeros_as_nodata = FALSE) {
    wbw_bundle_write_colour_composite(
      obj,
      style = "false_colour",
      output_path = output_path,
      red_key = red_key,
      green_key = green_key,
      blue_key = blue_key,
      key_types = key_types,
      enhance = enhance,
      treat_zeros_as_nodata = treat_zeros_as_nodata,
      session = session
    )
  }

  acquisition_datetime_utc <- function() {
    metadata()$acquisition_datetime_utc %||% NULL
  }

  processing_level <- function() {
    metadata()$processing_level %||% metadata()$product_level %||% NULL
  }

  tile_id <- function() {
    metadata()$tile_id %||% NULL
  }

  mission <- function() {
    metadata()$mission %||% NULL
  }

  product_type <- function() {
    metadata()$product_type %||% NULL
  }

  acquisition_mode <- function() {
    metadata()$acquisition_mode %||% NULL
  }

  cloud_cover_percent <- function() {
    metadata()$cloud_cover_percent %||% NULL
  }

  polarizations <- function() {
    metadata()$polarizations %||% list()
  }

  obj <- new.env(parent = emptyenv())
  obj$path <- input_path
  obj$session <- session
  obj$metadata <- metadata
  obj$bundle_root <- metadata()$bundle_root %||% input_path
  obj$family <- metadata()$family %||% NULL
  obj$list_band_keys <- list_band_keys
  obj$list_measurement_keys <- list_measurement_keys
  obj$list_qa_keys <- list_qa_keys
  obj$list_aux_keys <- list_aux_keys
  obj$list_asset_keys <- list_asset_keys
  obj$key_summary <- key_summary
  obj$resolve_key <- resolve_key
  obj$has_key <- has_key
  obj$read_any <- read_any
  obj$read_band <- read_band
  obj$read_measurement <- read_measurement
  obj$read_qa_layer <- read_qa_layer
  obj$read_aux_layer <- read_aux_layer
  obj$read_asset <- read_asset
  obj$read_preview_raster <- read_preview_raster
  obj$write_true_colour <- write_true_colour
  obj$write_false_colour <- write_false_colour
  obj$acquisition_datetime_utc <- acquisition_datetime_utc
  obj$processing_level <- processing_level
  obj$tile_id <- tile_id
  obj$mission <- mission
  obj$product_type <- product_type
  obj$acquisition_mode <- acquisition_mode
  obj$cloud_cover_percent <- cloud_cover_percent
  obj$polarizations <- polarizations
  class(obj) <- c("wbw_sensor_bundle", "wbw_data_object")
  obj
}

wbw_collect_tools <- function(floating_license_id = NULL,
                              include_pro = NULL,
                              tier = "open",
                              signed_entitlement_json = NULL,
                              entitlement_file = NULL,
                              public_key_kid = NULL,
                              public_key_b64url = NULL,
                              provider_url = NULL,
                              machine_id = NULL,
                              customer_id = NULL) {
  session <- wbw_build_session(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    signed_entitlement_json = signed_entitlement_json,
    entitlement_file = entitlement_file,
    public_key_kid = public_key_kid,
    public_key_b64url = public_key_b64url,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
  session$list_tools()
}

wbw_list_tools <- function(...) {
  stop(
    "wbw_list_tools() was removed in Phase 4. Use wbw_tool_ids(), wbw_search_tools(), or wbw_describe_tool().",
    call. = FALSE
  )
}

#' @export
wbw_describe_tool <- function(tool_id,
                              session = NULL,
                              floating_license_id = NULL,
                              include_pro = NULL,
                              tier = "open",
                              signed_entitlement_json = NULL,
                              entitlement_file = NULL,
                              public_key_kid = NULL,
                              public_key_b64url = NULL,
                              provider_url = NULL,
                              machine_id = NULL,
                              customer_id = NULL) {
  tools <- wbw_collect_tools(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    signed_entitlement_json = signed_entitlement_json,
    entitlement_file = entitlement_file,
    public_key_kid = public_key_kid,
    public_key_b64url = public_key_b64url,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )

  if (!is.null(session)) {
    tools <- session$list_tools()
  }

  matches <- Filter(function(t) identical(t$id %||% "", tool_id), tools)
  if (length(matches) == 0L) {
    stop(sprintf("Tool not found: %s", tool_id), call. = FALSE)
  }
  matches[[1]]
}

#' @export
wbw_search_tools <- function(query,
                             session = NULL,
                             floating_license_id = NULL,
                             include_pro = NULL,
                             tier = "open",
                             signed_entitlement_json = NULL,
                             entitlement_file = NULL,
                             public_key_kid = NULL,
                             public_key_b64url = NULL,
                             provider_url = NULL,
                             machine_id = NULL,
                             customer_id = NULL) {
  if (!nzchar(query)) {
    return(list())
  }

  tools <- wbw_collect_tools(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    signed_entitlement_json = signed_entitlement_json,
    entitlement_file = entitlement_file,
    public_key_kid = public_key_kid,
    public_key_b64url = public_key_b64url,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )

  if (!is.null(session)) {
    tools <- session$list_tools()
  }

  q <- tolower(query)
  Filter(function(t) {
    hay <- c(
      t$id %||% "",
      t$display_name %||% "",
      t$summary %||% "",
      t$category %||% "",
      unlist(t$tags %||% list(), use.names = FALSE)
    )
    any(grepl(q, tolower(hay), fixed = TRUE))
  }, tools)
}

#' @export
wbw_read_raster <- function(path, session = NULL, proxy = FALSE) {
  wbw_raster_from_path(path, session = session, proxy = proxy)
}

#' @export
wbw_read_vector <- function(path,
                            options = NULL,
                            strict_format_options = FALSE,
                            session = NULL) {
  wbw_vector_from_path(
    path,
    session = session,
    read_options = options,
    strict_format_options = strict_format_options
  )
}

#' @export
wbw_read_lidar <- function(path, session = NULL) {
  wbw_lidar_from_path(path, session = session)
}

#' @export
wbw_read_bundle <- function(path, session = NULL) {
  wbw_sensor_bundle_from_path(path, session = session)
}

#' @export
wbw_read_landsat <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "landsat")
}

#' @export
wbw_read_sentinel1 <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "sentinel1_safe")
}

#' @export
wbw_read_sentinel2 <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "sentinel2_safe")
}

#' @export
wbw_read_planetscope <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "planetscope")
}

#' @export
wbw_read_iceye <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "iceye")
}

#' @export
wbw_read_dimap <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "dimap")
}

#' @export
wbw_read_maxar_worldview <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "maxar_worldview")
}

#' @export
wbw_read_radarsat2 <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "radarsat2")
}

#' @export
wbw_read_rcm <- function(path, session = NULL) {
  wbw_bundle_expect_family(wbw_read_bundle(path, session = session), "rcm")
}

#' @export
wbw_tool_ids <- function(session = NULL,
                         floating_license_id = NULL,
                         include_pro = NULL,
                         tier = "open",
                         signed_entitlement_json = NULL,
                         entitlement_file = NULL,
                         public_key_kid = NULL,
                         public_key_b64url = NULL,
                         provider_url = NULL,
                         machine_id = NULL,
                         customer_id = NULL) {
  if (is.null(session)) {
    session <- wbw_build_session(
      floating_license_id = floating_license_id,
      include_pro = include_pro,
      tier = tier,
      signed_entitlement_json = signed_entitlement_json,
      entitlement_file = entitlement_file,
      public_key_kid = public_key_kid,
      public_key_b64url = public_key_b64url,
      provider_url = provider_url,
      machine_id = machine_id,
      customer_id = customer_id
    )
  }
  tools <- session$list_tools()
  vapply(tools, function(t) t$id %||% "", character(1))
}

#' @export
wbw_has_tool <- function(tool_id,
                         session = NULL,
                         floating_license_id = NULL,
                         include_pro = NULL,
                         tier = "open",
                         signed_entitlement_json = NULL,
                         entitlement_file = NULL,
                         public_key_kid = NULL,
                         public_key_b64url = NULL,
                         provider_url = NULL,
                         machine_id = NULL,
                         customer_id = NULL) {
  ids <- wbw_tool_ids(
    session = session,
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    signed_entitlement_json = signed_entitlement_json,
    entitlement_file = entitlement_file,
    public_key_kid = public_key_kid,
    public_key_b64url = public_key_b64url,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
  tool_id %in% ids
}

wbw_run_tool <- function(tool_id,
                         args = list(),
                         session = NULL,
                         floating_license_id = NULL,
                         include_pro = NULL,
                         tier = "open",
                         signed_entitlement_json = NULL,
                         entitlement_file = NULL,
                         public_key_kid = NULL,
                         public_key_b64url = NULL,
                         provider_url = NULL,
                         machine_id = NULL,
                         customer_id = NULL) {
  if (is.null(session)) {
    session <- wbw_build_session(
      floating_license_id = floating_license_id,
      include_pro = include_pro,
      tier = tier,
      signed_entitlement_json = signed_entitlement_json,
      entitlement_file = entitlement_file,
      public_key_kid = public_key_kid,
      public_key_b64url = public_key_b64url,
      provider_url = provider_url,
      machine_id = machine_id,
      customer_id = customer_id
    )
  }
  session$run_tool(tool_id, args)
}

#' @export
wbw_run_tool_with_progress <- function(tool_id,
                                       args = list(),
                                       session = NULL,
                                       floating_license_id = NULL,
                                       include_pro = NULL,
                                       tier = "open",
                                       signed_entitlement_json = NULL,
                                       entitlement_file = NULL,
                                       public_key_kid = NULL,
                                       public_key_b64url = NULL,
                                       provider_url = NULL,
                                       machine_id = NULL,
                                       customer_id = NULL,
                                       on_progress = NULL) {
  if (is.null(session)) {
    session <- wbw_build_session(
      floating_license_id = floating_license_id,
      include_pro = include_pro,
      tier = tier,
      signed_entitlement_json = signed_entitlement_json,
      entitlement_file = entitlement_file,
      public_key_kid = public_key_kid,
      public_key_b64url = public_key_b64url,
      provider_url = provider_url,
      machine_id = machine_id,
      customer_id = customer_id
    )
  }
  result <- session$run_tool_with_progress(tool_id, args)
  result <- wbw_coerce_progress_result(result, session = session)
  if (!is.null(on_progress) && is.function(on_progress)) {
    events <- result$progress
    if (is.list(events) && length(events) > 0L) {
      for (ev in events) {
        pct <- ev$pct %||% ev$percent %||% NA_real_
        msg <- ev$message %||% ev$msg %||% ""
        on_progress(pct, msg)
      }
    }
  }
  result
}

wbw_make_entitlement_session <- function(signed_entitlement_json,
                                         public_key_kid,
                                         public_key_b64url,
                                         include_pro = FALSE,
                                         fallback_tier = "open") {
  run_tool <- function(tool_id, args = list()) {
    args_json <- wbw_args_to_json(args)
    out_json <- run_tool_json_with_entitlement_options(
      tool_id,
      args_json,
      signed_entitlement_json,
      public_key_kid,
      public_key_b64url,
      include_pro,
      fallback_tier
    )
    out <- jsonlite::fromJSON(out_json, simplifyVector = FALSE)
    wbw_coerce_tool_output(out, session = session)
  }

  run_tool_with_progress <- function(tool_id, args = list()) {
    args_json <- wbw_args_to_json(args)
    if (!is.loaded("wrap__run_tool_json_with_progress_entitlement_options")) {
      return(wbw_progress_result_fallback(tool_id, run_tool(tool_id, args)))
    }
    out_json <- run_tool_json_with_progress_entitlement_options(
      tool_id,
      args_json,
      signed_entitlement_json,
      public_key_kid,
      public_key_b64url,
      include_pro,
      fallback_tier
    )
    out <- jsonlite::fromJSON(out_json, simplifyVector = FALSE)
    wbw_coerce_progress_result(out, session = session)
  }

  list_tools <- function() {
    out_json <- list_tools_json_with_entitlement_options(
      signed_entitlement_json,
      public_key_kid,
      public_key_b64url,
      include_pro,
      fallback_tier
    )
    jsonlite::fromJSON(out_json, simplifyVector = FALSE)
  }

  session <- new.env(parent = emptyenv())
  session$run_tool <- run_tool
  session$run_tool_with_progress <- run_tool_with_progress
  session$list_tools <- list_tools
  session
}

wbw_make_entitlement_file_session <- function(entitlement_file,
                                              public_key_kid,
                                              public_key_b64url,
                                              include_pro = FALSE,
                                              fallback_tier = "open") {
  run_tool <- function(tool_id, args = list()) {
    args_json <- wbw_args_to_json(args)
    out_json <- run_tool_json_with_entitlement_file_options(
      tool_id,
      args_json,
      entitlement_file,
      public_key_kid,
      public_key_b64url,
      include_pro,
      fallback_tier
    )
    out <- jsonlite::fromJSON(out_json, simplifyVector = FALSE)
    wbw_coerce_tool_output(out, session = session)
  }

  run_tool_with_progress <- function(tool_id, args = list()) {
    args_json <- wbw_args_to_json(args)
    if (!is.loaded("wrap__run_tool_json_with_progress_entitlement_file_options")) {
      return(wbw_progress_result_fallback(tool_id, run_tool(tool_id, args)))
    }
    out_json <- run_tool_json_with_progress_entitlement_file_options(
      tool_id,
      args_json,
      entitlement_file,
      public_key_kid,
      public_key_b64url,
      include_pro,
      fallback_tier
    )
    out <- jsonlite::fromJSON(out_json, simplifyVector = FALSE)
    wbw_coerce_progress_result(out, session = session)
  }

  list_tools <- function() {
    out_json <- list_tools_json_with_entitlement_file_options(
      entitlement_file,
      public_key_kid,
      public_key_b64url,
      include_pro,
      fallback_tier
    )
    jsonlite::fromJSON(out_json, simplifyVector = FALSE)
  }

  session <- new.env(parent = emptyenv())
  session$run_tool <- run_tool
  session$run_tool_with_progress <- run_tool_with_progress
  session$list_tools <- list_tools
  session
}

#' Convert a geospatial raster file into an R matrix or array.
#'
#' Single-band rasters return a matrix by default. Multiband rasters return
#' an array with dimensions (rows, cols, bands).
#'
#' @export
wbw_raster_to_array <- function(path, simplify_single_band = TRUE) {
  if (!requireNamespace("terra", quietly = TRUE)) {
    stop("wbw_raster_to_array() requires the 'terra' package.", call. = FALSE)
  }

  r <- terra::rast(path)
  rows <- terra::nrow(r)
  cols <- terra::ncol(r)
  bands <- terra::nlyr(r)
  vals <- terra::values(r, mat = TRUE)

  if (bands == 1L && isTRUE(simplify_single_band)) {
    return(matrix(vals[, 1], nrow = rows, ncol = cols, byrow = TRUE))
  }

  out <- array(NA_real_, dim = c(rows, cols, bands))
  for (b in seq_len(bands)) {
    out[, , b] <- matrix(vals[, b], nrow = rows, ncol = cols, byrow = TRUE)
  }
  out
}

#' @export
wbw_raster_to_stars <- function(path, proxy = FALSE) {
  if (!requireNamespace("stars", quietly = TRUE)) {
    stop("wbw_raster_to_stars() requires the 'stars' package.", call. = FALSE)
  }
  stars::read_stars(path, proxy = proxy)
}

#' Convert an R matrix/array into a geospatial raster file.
#'
#' If template_path is provided, extent and CRS are copied from the template.
#' For arrays, expected shape is (rows, cols, bands).
#'
#' @export
wbw_array_to_raster <- function(x,
                                output_path,
                                template_path = NULL,
                                overwrite = FALSE,
                                crs = NULL,
                                extent = NULL,
                                band_names = NULL) {
  if (!requireNamespace("terra", quietly = TRUE)) {
    stop("wbw_array_to_raster() requires the 'terra' package.", call. = FALSE)
  }

  if (file.exists(output_path) && !isTRUE(overwrite)) {
    stop("output_path exists; set overwrite=TRUE to replace it.", call. = FALSE)
  }

  if (is.matrix(x)) {
    r <- terra::rast(x)
  } else if (is.array(x) && length(dim(x)) == 3L) {
    dims <- dim(x)
    layers <- lapply(seq_len(dims[3]), function(i) terra::rast(x[, , i]))
    r <- do.call(terra::c, layers)
  } else {
    stop("x must be a matrix or a 3D array with dimensions (rows, cols, bands).", call. = FALSE)
  }

  if (!is.null(template_path)) {
    tmpl <- terra::rast(template_path)
    terra::ext(r) <- terra::ext(tmpl)
    terra::crs(r) <- terra::crs(tmpl)
  } else {
    if (!is.null(crs)) {
      terra::crs(r) <- crs
    }
    if (!is.null(extent)) {
      if (length(extent) != 4L) {
        stop("extent must be c(xmin, xmax, ymin, ymax).", call. = FALSE)
      }
      terra::ext(r) <- terra::ext(extent[1], extent[2], extent[3], extent[4])
    }
  }

  if (!is.null(band_names)) {
    names(r) <- band_names
  }

  terra::writeRaster(r, output_path, overwrite = overwrite)
  invisible(output_path)
}

#' @export
wbw_stars_to_raster <- function(x, output_path, overwrite = FALSE) {
  if (!requireNamespace("stars", quietly = TRUE)) {
    stop("wbw_stars_to_raster() requires the 'stars' package.", call. = FALSE)
  }
  if (file.exists(output_path)) {
    if (!isTRUE(overwrite)) {
      stop("output_path exists; set overwrite=TRUE to replace it.", call. = FALSE)
    }
    unlink(output_path)
  }
  stars::write_stars(x, output_path)
  invisible(output_path)
}

#' Get all available tool categories.
#'
#' Returns a character vector of unique tool categories.
#'
#' @param session Optional session object. If NULL, a default session is created.
#'
#' @export
wbw_get_all_categories <- function(session = NULL) {
  if (is.null(session)) {
    session <- wbw_session()
  }
  tools <- session$list_tools()
  categories <- unique(sapply(tools, function(x) x$category %||% "Other"))
  sort(categories)
}

#' Get tools organized by category.
#'
#' Lists all tools, organized by category with their display names and descriptions.
#'
#' @param session Optional session object. If NULL, a default session is created.
#'
#' @return A named list where each element is a category containing a data frame
#'         of tool id, display_name, and summary.
#'
#' @export
wbw_list_tools_by_category <- function(session = NULL) {
  if (is.null(session)) {
    session <- wbw_session()
  }
  tools <- session$list_tools()
  categories <- wbw_get_all_categories(session)

  result <- list()

  for (cat in categories) {
    cat_tools <- Filter(function(x) (x$category %||% "Other") == cat, tools)
    tool_data <- data.frame(
      id = sapply(cat_tools, function(x) x$id),
      display_name = sapply(cat_tools, function(x) x$display_name %||% ""),
      summary = sapply(cat_tools, function(x) x$summary %||% ""),
      license_tier = sapply(cat_tools, function(x) x$license_tier %||% "Open"),
      stability = sapply(cat_tools, function(x) x$stability %||% ""),
      row.names = NULL,
      stringsAsFactors = FALSE
    )
    result[[cat]] <- tool_data
  }

  result
}

#' Get tools in a specific category.
#'
#' Returns a data frame of all tools in the specified category.
#'
#' @param category The category name (e.g., "Raster", "Vector", "Lidar").
#' @param session Optional session object. If NULL, a default session is created.
#'
#' @return A data frame with columns: id, display_name, summary, license_tier, stability.
#'
#' @export
wbw_tools_in_category <- function(category, session = NULL) {
  if (!is.character(category) || length(category) != 1L) {
    stop("category must be a single string.", call. = FALSE)
  }
  if (is.null(session)) {
    session <- wbw_session()
  }
  tools <- session$list_tools()
  cat_tools <- Filter(function(x) (x$category %||% "Other") == category, tools)

  if (length(cat_tools) == 0L) {
    available <- paste(wbw_get_all_categories(session), collapse = ", ")
    stop(
      sprintf(
        "Category '%s' not found. Available categories: %s",
        category,
        available
      ),
      call. = FALSE
    )
  }

  data.frame(
    id = sapply(cat_tools, function(x) x$id),
    display_name = sapply(cat_tools, function(x) x$display_name %||% ""),
    summary = sapply(cat_tools, function(x) x$summary %||% ""),
    license_tier = sapply(cat_tools, function(x) x$license_tier %||% "Open"),
    stability = sapply(cat_tools, function(x) x$stability %||% ""),
    row.names = NULL,
    stringsAsFactors = FALSE
  )
}

#' Show tool category summary.
#'
#' Displays counts and statistics for each tool category.
#'
#' @param session Optional session object. If NULL, a default session is created.
#'
#' @return A data frame with category name and tool count.
#'
#' @export
wbw_category_summary <- function(session = NULL) {
  if (is.null(session)) {
    session <- wbw_session()
  }
  tools <- session$list_tools()
  categories <- wbw_get_all_categories(session)

  counts <- sapply(categories, function(cat) {
    length(Filter(function(x) (x$category %||% "Other") == cat, tools))
  })

  data.frame(
    category = names(counts),
    tool_count = as.integer(counts),
    row.names = NULL,
    stringsAsFactors = FALSE
  )
}
