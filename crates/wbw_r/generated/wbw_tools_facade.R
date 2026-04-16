# Stable facade over generated wbw_r wrappers.
#
# Source this file after loading the native wbw_r bindings and the generated
# wrapper module file. This keeps user code pointed at a small stable surface
# while generated wrapper internals can be refreshed as tool manifests evolve.

source("crates/wbw_r/generated/wbw_tools_generated.R")

wbw_session <- function(floating_license_id = NULL,
                        include_pro = NULL,
                        tier = "open",
                        provider_url = NULL,
                        machine_id = NULL,
                        customer_id = NULL) {
  wbw_make_session(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
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

wbw_list_tools <- function(floating_license_id = NULL,
                           include_pro = NULL,
                           tier = "open",
                           provider_url = NULL,
                           machine_id = NULL,
                           customer_id = NULL) {
  stop(
    "wbw_list_tools() was removed in Phase 4. Use wbw_tool_ids(), wbw_search_tools(), or wbw_describe_tool().",
    call. = FALSE
  )
}

wbw_run_tool <- function(tool_id,
                         args = list(),
                         floating_license_id = NULL,
                         include_pro = NULL,
                         tier = "open",
                         provider_url = NULL,
                         machine_id = NULL,
                         customer_id = NULL) {
  session <- wbw_session(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
  session$run_tool(tool_id, args)
}

lidar_change_and_disturbance_analysis <- function(baseline_tiles,
                                                  monitor_tiles,
                                                  resolution = NULL,
                                                  min_change_m = NULL,
                                                  output_prefix = NULL,
                                                  floating_license_id = NULL,
                                                  include_pro = NULL,
                                                  tier = "open",
                                                  provider_url = NULL,
                                                  machine_id = NULL,
                                                  customer_id = NULL) {
  wbw_run_tool(
    "lidar_change_and_disturbance_analysis",
    args = Filter(Negate(is.null), list(
      baseline_tiles = baseline_tiles,
      monitor_tiles = monitor_tiles,
      resolution = resolution,
      min_change_m = min_change_m,
      output_prefix = output_prefix
    )),
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}

sidewalk_vegetation_accessibility_monitoring <- function(lidar_tiles,
                                                         sidewalks,
                                                         sidewalks_epsg = NULL,
                                                         resolution = NULL,
                                                         segment_length_m = NULL,
                                                         clearance_height_m = NULL,
                                                         buffer_distance_m = NULL,
                                                         output_prefix = NULL,
                                                         floating_license_id = NULL,
                                                         include_pro = NULL,
                                                         tier = "open",
                                                         provider_url = NULL,
                                                         machine_id = NULL,
                                                         customer_id = NULL) {
  wbw_run_tool(
    "sidewalk_vegetation_accessibility_monitoring",
    args = Filter(Negate(is.null), list(
      lidar_tiles = lidar_tiles,
      sidewalks = sidewalks,
      sidewalks_epsg = sidewalks_epsg,
      resolution = resolution,
      segment_length_m = segment_length_m,
      clearance_height_m = clearance_height_m,
      buffer_distance_m = buffer_distance_m,
      output_prefix = output_prefix
    )),
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}

terrain_constraint_and_conflict_analysis <- function(dem,
                                                     wetness = NULL,
                                                     flood_risk = NULL,
                                                     landcover_penalty = NULL,
                                                     slope_limit_deg = NULL,
                                                     output_prefix = NULL,
                                                     floating_license_id = NULL,
                                                     include_pro = NULL,
                                                     tier = "open",
                                                     provider_url = NULL,
                                                     machine_id = NULL,
                                                     customer_id = NULL) {
  wbw_run_tool(
    "terrain_constraint_and_conflict_analysis",
    args = Filter(Negate(is.null), list(
      dem = dem,
      wetness = wetness,
      flood_risk = flood_risk,
      landcover_penalty = landcover_penalty,
      slope_limit_deg = slope_limit_deg,
      output_prefix = output_prefix
    )),
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}

terrain_constructability_and_cost_analysis <- function(dem,
                                                       existing_conflict = NULL,
                                                       wetness = NULL,
                                                       access_cost = NULL,
                                                       output_prefix = NULL,
                                                       floating_license_id = NULL,
                                                       include_pro = NULL,
                                                       tier = "open",
                                                       provider_url = NULL,
                                                       machine_id = NULL,
                                                       customer_id = NULL) {
  wbw_run_tool(
    "terrain_constructability_and_cost_analysis",
    args = Filter(Negate(is.null), list(
      dem = dem,
      existing_conflict = existing_conflict,
      wetness = wetness,
      access_cost = access_cost,
      output_prefix = output_prefix
    )),
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}

in_season_crop_stress_intervention_planning <- function(ndvi,
                                                        canopy_temperature = NULL,
                                                        soil_moisture = NULL,
                                                        output_prefix = NULL,
                                                        floating_license_id = NULL,
                                                        include_pro = NULL,
                                                        tier = "open",
                                                        provider_url = NULL,
                                                        machine_id = NULL,
                                                        customer_id = NULL) {
  wbw_run_tool(
    "in_season_crop_stress_intervention_planning",
    args = Filter(Negate(is.null), list(
      ndvi = ndvi,
      canopy_temperature = canopy_temperature,
      soil_moisture = soil_moisture,
      output_prefix = output_prefix
    )),
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}

field_trafficability_and_operation_planning <- function(dem,
                                                        soil_moisture,
                                                        rainfall_forecast = NULL,
                                                        output_prefix = NULL,
                                                        floating_license_id = NULL,
                                                        include_pro = NULL,
                                                        tier = "open",
                                                        provider_url = NULL,
                                                        machine_id = NULL,
                                                        customer_id = NULL) {
  wbw_run_tool(
    "field_trafficability_and_operation_planning",
    args = Filter(Negate(is.null), list(
      dem = dem,
      soil_moisture = soil_moisture,
      rainfall_forecast = rainfall_forecast,
      output_prefix = output_prefix
    )),
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}
