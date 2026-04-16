"""
Stream A-C Cookbook: Vehicle Routing & Multimodal Workflows

Demonstrates Stream A (CVRP), Stream B (VRPTW + Pickup/Delivery), and Stream C (Multimodal)
tools through practical use-cases.
"""

import geopandas as gpd
from pathlib import Path
import json
from wbw import WbEnvironment


def cookbook_stream_a_cvrp_municipal_delivery():
    """
    Stream A: CVRP for municipal waste collection/delivery routing
    
    Scenario: City courier service needs to deliver parcels from central depot to 20 customers.
    Constraints: Vehicles have 500kg capacity, must return to depot.
    """
    wbe = WbEnvironment()
    
    # Load network (road network layer)
    network = wbe.read_vector("data/city_street_network.gpkg")
    
    # Load customer stops with demand
    customers = wbe.read_vector("data/customer_delivery_points.gpkg")
    
    # Load depot(s)
    depot = wbe.read_vector("data/delivery_depot.gpkg")
    
    # Run CVRP tool
    result = wbe.run_tool(
        "vehicle_routing_cvrp",
        {
            "network": "data/city_street_network.gpkg",
            "demand_layer": "data/customer_delivery_points.gpkg",
            "depot_layer": "data/delivery_depot.gpkg",
            "demand_field": "DEMAND_KG",
            "vehicle_capacity": 500.0,
            "vehicle_cost_per_km": 1.5,
            "output_routes": "output/routes_municipal.gpkg",
            "output_assignment": "output/assignments_municipal.gpkg"
        }
    )
    
    print(f"CVRP Results:")
    print(f"  Vehicles used: {result['vehicle_count']}")
    print(f"  Served stops: {result['served_stop_count']}")
    print(f"  Unserved stops: {result['unserved_stop_count']}")
    print(f"  Total distance: {result.get('total_distance', 'N/A')} km")
    
    return result


def cookbook_stream_b_vrptw_urgent_service():
    """
    Stream B: VRPTW (Time Windows) for urgent/appointment-based service routing
    
    Scenario: Emergency HVAC repair service with 15 urgent calls throughout the day.
    Constraints: Each call has a 2-hour time window, service takes 30-60 minutes.
    """
    wbe = WbEnvironment()
    
    # Run VRPTW tool with time windows
    result = wbe.run_tool(
        "vehicle_routing_vrptw",
        {
            "network": "data/city_street_network.gpkg",
            "stops_layer": "data/hvac_emergency_calls.gpkg",
            "depot_layer": "data/hvac_depot.gpkg",
            "time_window_start_field": "CALL_TIME_MIN",
            "time_window_end_field": "CALL_TIME_MAX",
            "service_time_field": "SERVICE_MINUTES",
            "speed_kmh": 60.0,
            "vehicle_count": 4,
            "enforce_time_windows": True,  # Strict enforcement
            "allowed_lateness": 0,  # No flexibility
            "output_routes": "output/routes_hvac.gpkg",
            "output_diagnostics": "output/hvac_diagnostics.csv"
        }
    )
    
    print(f"VRPTW Results:")
    print(f"  Feasible routes: {result['feasible_route_count']}")
    print(f"  Late stops: {result['time_window_infeasible_stop_count']}")
    print(f"  Total lateness: {result['total_lateness']} minutes")
    
    return result


def cookbook_stream_b_pickup_delivery_logistics():
    """
    Stream B: Pickup/Delivery for 3PL (Third-Party Logistics) operations
    
    Scenario: Regional distribution center consolidates shipments from suppliers,
    redistributes to retailers. Pickup/delivery pairs must maintain order (pickup before delivery).
    """
    wbe = WbEnvironment()
    
    result = wbe.run_tool(
        "vehicle_routing_pickup_delivery",
        {
            "network": "data/regional_highway_network.gpkg",
            "requests_layer": "data/logistics_pickup_delivery_requests.gpkg",
            "depot_layer": "data/distribution_center.gpkg",
            "request_id_field": "SHIPMENT_ID",
            "stop_type_field": "STOP_TYPE",  # 'PICKUP' or 'DELIVERY'
            "demand_field": "WEIGHT_KG",
            "vehicle_capacity": 10000.0,
            "speed_kmh": 80.0,
            "vehicle_cost_per_km": 2.0,
            "output_routes": "output/routes_logistics.gpkg",
            "output_assignment": "output/assignments_logistics.gpkg"
        }
    )
    
    print(f"Pickup/Delivery Results:")
    print(f"  Served requests: {result['served_request_count']}")
    print(f"  Unserved requests: {result['unserved_request_count']}")
    print(f"  Infeasible requests: {result['infeasible_request_count']}")
    
    return result


def cookbook_stream_c_multimodal_urban_commute():
    """
    Stream C: Multimodal Shortest Path - Urban commute optimization
    
    Scenario: Calculate cost-minimizing route for commute using walk, bike, transit, and car.
    Different modes have different speeds; transfers between modes incur penalties.
    """
    wbe = WbEnvironment()
    
    result = wbe.run_tool(
        "multimodal_shortest_path",
        {
            "network": "data/multimodal_network.gpkg",
            "origin": "data/home_location.gpkg",
            "destination": "data/office_location.gpkg",
            "mode_field": "TRANSPORT_MODE",  # 'walk', 'bike', 'transit', 'car'
            "speed_field": "SPEED_KMH",
            "mode_speed_overrides": json.dumps({
                "walk": 5,
                "bike": 20,
                "transit": 40,
                "car": 60
            }),
            "allowed_modes": "walk,bike,transit,car",
            "transfer_penalty": 300,  # 5-minute transfer penalty (in seconds)
            "output_path": "output/commute_route.gpkg"
        }
    )
    
    print(f"Multimodal Route Results:")
    print(f"  Total cost: {result['cost']} (seconds)")
    print(f"  Mode changes: {result['mode_changes']}")
    print(f"  Transfer penalties: {result['transfer_penalty']} seconds")
    
    return result


def cookbook_stream_c_multimodal_walk_transit():
    """
    Stream C: Walk-First Transit - Constrain to walking and transit only
    
    Scenario: Calculate pedestrian-friendly route using public transit.
    Useful for accessibility planning or car-free routing.
    """
    wbe = WbEnvironment()
    
    result = wbe.run_tool(
        "multimodal_shortest_path",
        {
            "network": "data/multimodal_network.gpkg",
            "origin": "data/neighborhood_start.gpkg",
            "destination": "data/downtown_destination.gpkg",
            "mode_field": "TRANSPORT_MODE",
            "allowed_modes": "walk,transit",  # Walk + Transit only
            "transfer_penalty": 120,  # 2-minute walk to transit penalty
            "output_path": "output/walk_transit_route.gpkg"
        }
    )
    
    print(f"Walk-Transit Route:")
    print(f"  Total cost: {result['cost']}")
    print(f"  Mode changes: {result['mode_changes']}")
    
    return result


if __name__ == "__main__":
    print("=" * 70)
    print("Stream A-C Vehicle Routing & Multimodal Cookbook")
    print("=" * 70)
    
    print("\n[Stream A] CVRP Municipal Delivery")
    print("-" * 70)
    cookbook_stream_a_cvrp_municipal_delivery()
    
    print("\n[Stream B] VRPTW Urgent Service Routing")
    print("-" * 70)
    cookbook_stream_b_vrptw_urgent_service()
    
    print("\n[Stream B] Pickup/Delivery Logistics")
    print("-" * 70)
    cookbook_stream_b_pickup_delivery_logistics()
    
    print("\n[Stream C] Multimodal Commute Journey")
    print("-" * 70)
    cookbook_stream_c_multimodal_urban_commute()
    
    print("\n[Stream C] Walk-Transit Network")
    print("-" * 70)
    cookbook_stream_c_multimodal_walk_transit()
    
    print("\n" + "=" * 70)
    print("Cookbook examples complete!")
    print("=" * 70)
