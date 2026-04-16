# Stream A-D Cookbook: Vehicle Routing and Network Analytics with whiteboxworkflows
# 
# Demonstrates usage of Phase 3 tools (Stream A-D) in R:
# - Stream A: CVRP (Capacitated Vehicle Routing)
# - Stream B: VRPTW (Vehicle Routing with Time Windows) & Pickup/Delivery
# - Stream C: Multimodal Shortest Path
# - Stream D: Network Centrality, Accessibility, OD Sensitivity

library(whiteboxworkflows)

# Initialize session
session <- wbw_session()

# =========================================================================
# Stream A: CVRP Example - Municipal Delivery Service
# =========================================================================
cookbook_stream_a_cvrp <- function(session = wbw_session()) {
  cat("=== Stream A: Capacitated Vehicle Routing (CVRP) ===\n")
  cat("Scenario: Deliver packages from central depot to 20 customer locations.\n")
  cat("Vehicle capacity: 500 kg, Cost: $1.50/km\n\n")
  
  result <- wbw_run_tool(
    session = session,
    tool_id = "vehicle_routing_cvrp",
    args = list(
      input = "data/city_network.gpkg",
      demand_layer = "data/customers.gpkg",
      depot_layer = "data/depot.gpkg",
      demand_field = "DEMAND_KG",
      vehicle_capacity = 500.0,
      vehicle_cost_per_km = 1.5,
      output = "output/cvrp_routes.gpkg",
      assignment_output = "output/cvrp_assignments.gpkg"
    )
  )
  
  cat("CVRP Results:\n")
  cat(sprintf("  Routes created: %d\n", result$route_count))
  cat(sprintf("  Served customers: %d\n", result$served_stop_count))
  cat(sprintf("  Unserved customers: %d\n", result$unserved_stop_count))
  cat("\n")
  
  return(result)
}

# =========================================================================
# Stream B: VRPTW Example - Time Window Constraints
# =========================================================================
cookbook_stream_b_vrptw <- function(session = wbw_session()) {
  cat("=== Stream B: Vehicle Routing with Time Windows (VRPTW) ===\n")
  cat("Scenario: HVAC emergency dispatching with 2-hour service windows.\n")
  cat("Flexible lateness allowed: 30 minutes\n\n")
  
  result <- wbw_run_tool(
    session = session,
    tool_id = "vehicle_routing_vrptw",
    args = list(
      input = "data/city_network.gpkg",
      stops = "data/hvac_calls.gpkg",
      depot = "data/hvac_depot.gpkg",
      time_window_start_field = "CALL_TIME_MIN",
      time_window_end_field = "CALL_TIME_MAX",
      service_time_field = "SERVICE_MIN",
      enforce_time_windows = FALSE,
      allowed_lateness = 30.0,
      output = "output/vrptw_routes.gpkg"
    )
  )
  
  cat("VRPTW Results:\n")
  cat(sprintf("  Feasible routes: %d\n", result$feasible_route_count))
  cat(sprintf("  Late stops: %d\n", result$late_stop_count))
  cat(sprintf("  Total lateness: %d minutes\n", result$total_lateness))
  cat("\n")
  
  return(result)
}

# =========================================================================
# Stream B: Pickup/Delivery Example  
# =========================================================================
cookbook_stream_b_pickup_delivery <- function(session = wbw_session()) {
  cat("=== Stream B: Pickup/Delivery Routing ===\n")
  cat("Scenario: 3PL consolidation center routes shipments to retail partners.\n")
  cat("Constraint: Pickup before delivery for each shipment\n\n")
  
  result <- wbw_run_tool(
    session = session,
    tool_id = "vehicle_routing_pickup_delivery",
    args = list(
      input = "data/regional_network.gpkg",
      requests = "data/shipments.gpkg",
      depot = "data/dc.gpkg",
      request_id_field = "SHIPMENT_ID",
      stop_type_field = "STOP_TYPE",
      demand_field = "WEIGHT_KG",
      vehicle_capacity = 10000.0,
      output = "output/pd_routes.gpkg"
    )
  )
  
  cat("Pickup/Delivery Results:\n")
  cat(sprintf("  Served requests: %d\n", result$served_request_count))
  cat(sprintf("  Unserved requests: %d\n", result$unserved_request_count))
  cat("\n")
  
  return(result)
}

# =========================================================================
# Stream C: Multimodal Shortest Path Example
# =========================================================================
cookbook_stream_c_multimodal <- function(session = wbw_session()) {
  cat("=== Stream C: Multimodal Shortest Path ===\n")
  cat("Scenario: Commute routing with walk, bike, transit, and car options.\n")
  cat("Transfer penalty: 5 minutes between modes\n\n")
  
  mode_speeds <- list(
    walk = 5,
    bike = 20,
    transit = 40,
    car = 60
  )
  
  result <- wbw_run_tool(
    session = session,
    tool_id = "multimodal_shortest_path",
    args = list(
      input = "data/multimodal_network.gpkg",
      origin = "data/home.gpkg",
      destination = "data/office.gpkg",
      mode_field = "TRANSPORT_MODE",
      speed_field = "SPEED_KMH",
      mode_speed_overrides = jsonlite::toJSON(mode_speeds),
      allowed_modes = "walk,bike,transit,car",
      transfer_penalty = 300.0,
      output = "output/commute_route.gpkg"
    )
  )
  
  cat("Multimodal Route Results:\n")
  cat(sprintf("  Route cost: %f seconds\n", result$cost))
  cat(sprintf("  Mode changes: %d\n", result$mode_changes))
  cat("\n")
  
  return(result)
}

# =========================================================================
# Stream D: Network Centrality Example
# =========================================================================
cookbook_stream_d_centrality <- function(session = wbw_session()) {
  cat("=== Stream D: Network Centrality Metrics ===\n")
  cat("Scenario: Identify critical intersections for emergency service placement.\n\n")
  
  result <- wbw_run_tool(
    session = session,
    tool_id = "network_centrality_metrics",
    args = list(
      input = "data/city_network.gpkg",
      edge_cost_field = "LENGTH_M",
      output = "output/centrality.gpkg"
    )
  )
  
  # Read results and find top nodes
  centrality_nodes <- sf::read_sf("output/centrality.gpkg")
  
  # Top by betweenness (traffic flow)
  top_between <- head(centrality_nodes[order(centrality_nodes$BETWEENNESS, decreasing = TRUE), ], 5)
  
  cat("Network Centrality Results:\n")
  cat(sprintf("  Total nodes analyzed: %d\n", nrow(centrality_nodes)))
  cat("  Top 5 nodes by betweenness (flow):\n")
  for (i in 1:min(5, nrow(top_between))) {
    cat(sprintf("    %d. Node %d (betweenness: %.2f)\n", 
                i, top_between$FID[i], top_between$BETWEENNESS[i]))
  }
  cat("\n")
  
  return(result)
}

# =========================================================================
# Stream D: Accessibility Metrics Example
# =========================================================================
cookbook_stream_d_accessibility <- function(session = wbw_session()) {
  cat("=== Stream D: Accessibility Metrics ===\n")
  cat("Scenario: Evaluate retail site accessibility to population centers.\n")
  cat("Cutoff: 15-minute drive, Exponential decay function\n\n")
  
  result <- wbw_run_tool(
    session = session,
    tool_id = "network_accessibility_metrics",
    args = list(
      input = "data/street_network.gpkg",
      origins = "data/retail_sites.gpkg",
      destinations = "data/population_centers.gpkg",
      impedance_cutoff = 15.0,
      decay_function = "exponential",
      decay_parameter = 0.3,
      edge_cost_field = "TRAVEL_TIME_MIN",
      output = "output/accessibility.gpkg"
    )
  )
  
  # Read and analyze
  accessibility <- sf::read_sf("output/accessibility.gpkg")
  
  cat("Accessibility Results:\n")
  cat(sprintf("  Sites evaluated: %d\n", nrow(accessibility)))
  cat(sprintf("  Mean accessibility: %.2f\n", mean(accessibility$ACCESSIBILITY)))
  cat(sprintf("  Best site accessibility: %.2f\n", max(accessibility$ACCESSIBILITY)))
  cat("\n")
  
  return(result)
}

# =========================================================================
# Stream D: OD Sensitivity Example
# =========================================================================
cookbook_stream_d_od_sensitivity <- function(session = wbw_session()) {
  cat("=== Stream D: OD Sensitivity Analysis ===\n")
  cat("Scenario: Quantify freight routing cost uncertainty (±15% variation).\n")
  cat("Monte Carlo samples: 100\n\n")
  
  result <- wbw_run_tool(
    session = session,
    tool_id = "od_sensitivity_analysis",
    args = list(
      input = "data/freight_network.gpkg",
      origins = "data/dcs.gpkg",
      destinations = "data/warehouses.gpkg",
      edge_cost_field = "COST_PER_KM",
      impedance_disturbance_range = "0.85,1.15",
      monte_carlo_samples = 100L,
      output = "output/od_sensitivity.csv"
    )
  )
  
  # Read and analyze CSV
  sensitivity <- read.csv("output/od_sensitivity.csv")
  
  cat("OD Sensitivity Results:\n")
  cat(sprintf("  OD pairs: %d\n", nrow(sensitivity)))
  cat(sprintf("  Mean cost variability (stdev): %.2f\n", mean(sensitivity$stdev_cost)))
  cat(sprintf("  Max observed range: %.2f\n", 
              max(sensitivity$max_cost - sensitivity$min_cost)))
  cat(sprintf("  Most uncertain route stdev: %.2f\n", max(sensitivity$stdev_cost)))
  cat("\n")
  
  return(result)
}

# =========================================================================
# Main execution
# =========================================================================
main <- function() {
  cat("\n")
  cat("======================================================================\n")
  cat("Stream A-D Cookbook: Vehicle Routing and Network Analytics\n")
  cat("======================================================================\n")
  cat("\n")
  
  # Stream A
  cookbook_stream_a_cvrp(session)
  
  # Stream B
  cookbook_stream_b_vrptw(session)
  cookbook_stream_b_pickup_delivery(session)
  
  # Stream C
  cookbook_stream_c_multimodal(session)
  
  # Stream D
  cookbook_stream_d_centrality(session)
  cookbook_stream_d_accessibility(session)
  cookbook_stream_d_od_sensitivity(session)
  
  cat("======================================================================\n")
  cat("Cookbook complete!\n")
  cat("======================================================================\n")
}

# Execute
main()
