# Stream C Cookbook: Batched multimodal temporal scenario analysis
#
# Demonstrates comparative multimodal OD and route workflows using
# temporal cost profiles and scenario bundles.

library(whiteboxworkflows)
library(readr)
library(dplyr)
library(sf)

session <- wbw_session()

cookbook_multimodal_od_temporal_scenarios <- function(session = wbw_session()) {
  cat("=== Stream C: Multimodal OD Cost Matrix with Temporal Scenarios ===\n")
  cat("Scenario: Compare off-peak and peak commute impedance across many OD pairs.\n\n")

  result <- wbw_run_tool(
    session = session,
    tool_id = "multimodal_od_cost_matrix",
    args = list(
      input = "data/multimodal_network.gpkg",
      origins = "data/commuter_origins.gpkg",
      destinations = "data/employment_centers.gpkg",
      mode_field = "MODE",
      allowed_modes = "walk,transit,drive",
      mode_speed_overrides = "walk:1.3,transit:7.5,drive:12.0",
      transfer_penalty = 0.2,
      scenario_bundle_csv = "data/multimodal_temporal_scenarios.csv",
      output = "output/multimodal_temporal_od.csv"
    )
  )

  od <- read_csv("output/multimodal_temporal_od.csv", show_col_types = FALSE)
  summary <- od %>%
    filter(reachable) %>%
    group_by(scenario) %>%
    summarise(
      od_pairs = n(),
      mean_cost = mean(cost),
      max_cost = max(cost),
      .groups = "drop"
    )

  cat("Temporal OD Summary:\n")
  print(summary)
  cat(sprintf("Scenarios processed: %d\n", result$scenario_count))
  cat(sprintf("Reachable pairs: %d\n\n", result$reachable_pair_count))

  invisible(result)
}

cookbook_multimodal_routes_temporal_comparison <- function(session = wbw_session()) {
  cat("=== Stream C: Multimodal Routes from OD with Temporal Scenarios ===\n")
  cat("Scenario: Export comparable route geometries for peak and off-peak service plans.\n\n")

  result <- wbw_run_tool(
    session = session,
    tool_id = "multimodal_routes_from_od",
    args = list(
      input = "data/multimodal_network.gpkg",
      origins = "data/key_origins.gpkg",
      destinations = "data/key_destinations.gpkg",
      mode_field = "MODE",
      allowed_modes = "walk,transit,drive",
      mode_speed_overrides = "walk:1.3,transit:7.5,drive:12.0",
      transfer_penalty = 0.2,
      scenario_bundle_csv = "data/multimodal_temporal_scenarios.csv",
      output = "output/multimodal_temporal_routes.gpkg"
    )
  )

  routes <- read_sf("output/multimodal_temporal_routes.gpkg")
  summary <- routes %>%
    st_drop_geometry() %>%
    group_by(SCENARIO) %>%
    summarise(
      route_count = n(),
      mean_cost = mean(COST),
      mean_mode_changes = mean(MODE_CHG),
      .groups = "drop"
    )

  cat("Temporal Route Summary:\n")
  print(summary)
  cat(sprintf("Routes written: %d\n\n", result$route_count))

  invisible(result)
}

if (sys.nframe() == 0) {
  cat(strrep("=", 72), "\n", sep = "")
  cat("Stream C Multimodal Batch Temporal Cookbook\n")
  cat(strrep("=", 72), "\n", sep = "")
  cookbook_multimodal_od_temporal_scenarios(session)
  cat(strrep("-", 72), "\n", sep = "")
  cookbook_multimodal_routes_temporal_comparison(session)
}
