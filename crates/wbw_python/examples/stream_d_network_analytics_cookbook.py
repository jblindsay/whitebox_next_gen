"""
Stream D Cookbook: Network Analytics (Centrality, Accessibility, OD Sensitivity)

Demonstrates Stream D tools for network analysis, accessibility measurement, and
origin-destination uncertainty quantification.
"""

import geopandas as gpd
from pathlib import Path
import json
from wbw import WbEnvironment


def cookbook_stream_d_centrality_urban_planning():
    """
    Stream D: Network Centrality - Identify key intersections for urban planning
    
    Scenario: City planner wants to identify critical intersections for placing
    emergency services, commercial centers, or transit hubs. Uses betweenness,
    closeness, and degree centrality to rank nodes.
    """
    wbe = WbEnvironment()
    
    # Run network centrality metrics
    result = wbe.run_tool(
        "network_centrality_metrics",
        {
            "input": "data/city_street_network.gpkg",
            "edge_cost_field": "LENGTH_M",
            "one_way_field": "ONE_WAY",
            "blocked_field": "BLOCKED",
            "output": "output/centrality_metrics.gpkg"
        }
    )
    
    # Load results and rank nodes
    centrality_nodes = gpd.read_file("output/centrality_metrics.gpkg")
    
    # Sort by betweenness (flow through intersection)
    top_betweenness = centrality_nodes.nlargest(10, "BETWEENNESS")
    print(f"Top 10 Most Central Intersections (Betweenness):")
    print(f"  Junction IDs: {top_betweenness.index.tolist()}")
    
    # Sort by closeness (average distance to all other nodes)
    top_closeness = centrality_nodes.nlargest(10, "CLOSENESS")
    print(f"\nTop 10 Most Connected Intersections (Closeness):")
    print(f"  Junction IDs: {top_closeness.index.tolist()}")
    
    # Sort by degree (direct connections)
    top_degree = centrality_nodes.nlargest(10, "DEGREE")
    print(f"\nTop 10 Highest Degree Intersections:")
    print(f"  Junction IDs: {top_degree.index.tolist()}")
    
    return result, centrality_nodes


def cookbook_stream_d_accessibility_retail_site_selection():
    """
    Stream D: Accessibility Metrics - Evaluate retail site accessibility
    
    Scenario: Retail chain analyzes potential store locations based on accessibility
    to population centers. Uses impedance cutoff (e.g., 15-minute drive) and decay
    function to weight closer populations higher.
    """
    wbe = WbEnvironment()
    
    # Test with exponential decay (standard preference in retail)
    result = wbe.run_tool(
        "network_accessibility_metrics",
        {
            "input": "data/street_network.gpkg",
            "origins": "data/candidate_retail_sites.gpkg",
            "destinations": "data/residential_population_centers.gpkg",
            "impedance_cutoff": 15.0,  # 15-minute drive
            "decay_function": "exponential",
            "decay_parameter": 0.3,  # Lambda for exp(-0.3 * distance)
            "edge_cost_field": "TRAVEL_TIME_MIN",
            "output": "output/retail_accessibility.gpkg"
        }
    )
    
    # Load and analyze
    accessibility = gpd.read_file("output/retail_accessibility.gpkg")
    
    print(f"Accessibility Analysis Results:")
    print(f"  Best site: {accessibility['ACCESSIBILITY'].idxmax()}")
    print(f"  Max accessibility score: {accessibility['ACCESSIBILITY'].max():.2f}")
    print(f"  Mean accessibility: {accessibility['ACCESSIBILITY'].mean():.2f}")
    print(f"  Site rankings:\n{accessibility[['NAME', 'ACCESSIBILITY']].sort_values('ACCESSIBILITY', ascending=False).head(5)}")
    
    return result


def cookbook_stream_d_accessibility_healthcare_equity():
    """
    Stream D: Healthcare Equity - Measure accessibility to clinics
    
    Scenario: Public health department evaluates geographic health equity by measuring
    distance-based accessibility from residential areas to healthcare facilities.
    Linear decay emphasizes all destinations equally within cutoff.
    """
    wbe = WbEnvironment()
    
    # Linear decay - all destinations within range weighted equally, then decay
    result = wbe.run_tool(
        "network_accessibility_metrics",
        {
            "input": "data/street_network_bike_friendly.gpkg",
            "origins": "data/census_block_centers.gpkg",
            "destinations": "data/clinics.gpkg",
            "impedance_cutoff": 30.0,  # 30-minute walk/bike
            "decay_function": "linear",
            "decay_parameter": 1.0,  # Linear penalty
            "edge_cost_field": "TRAVEL_TIME_MIN",
            "output": "output/healthcare_accessibility.gpkg"
        }
    )
    
    accessibility = gpd.read_file("output/healthcare_accessibility.gpkg")
    
    # Identify underserved areas
    low_access = accessibility[accessibility['ACCESSIBILITY'] < accessibility['ACCESSIBILITY'].quantile(0.25)]
    print(f"Healthcare Accessibility Results:")
    print(f"  Total census blocks: {len(accessibility)}")
    print(f"  Underserved blocks (bottom quartile): {len(low_access)}")
    print(f"  Mean accessibility: {accessibility['ACCESSIBILITY'].mean():.2f}")
    
    return result


def cookbook_stream_d_od_sensitivity_cost_uncertainty():
    """
    Stream D: OD Sensitivity Analysis - Quantify cost uncertainty
    
    Scenario: Transportation planner wants to understand how freight costs vary
    based on traffic/congestion uncertainty. Performs Monte Carlo sampling of
    edge costs within ±15% range (morning peak vs off-peak variation).
    """
    wbe = WbEnvironment()
    
    result = wbe.run_tool(
        "od_sensitivity_analysis",
        {
            "input": "data/freight_network.gpkg",
            "origins": "data/distribution_centers.gpkg",
            "destinations": "data/warehouses.gpkg",
            "edge_cost_field": "COST_PER_KM",
            "impedance_disturbance_range": "0.85,1.15",  # ±15% variation
            "monte_carlo_samples": 100,
            "output": "output/freight_od_sensitivity.csv"
        }
    )
    
    # Load and analyze results
    import pandas as pd
    sensitivity = pd.read_csv("output/freight_od_sensitivity.csv")
    
    print(f"OD Sensitivity Analysis Results:")
    print(f"  OD pairs analyzed: {len(sensitivity)}")
    print(f"  Mean cost variability (stdev): {sensitivity['stdev_cost'].mean():.2f}")
    print(f"  Max cost range observed: {(sensitivity['max_cost'] - sensitivity['min_cost']).max():.2f}")
    
    # Identify high-uncertainty routes
    high_uncertainty = sensitivity[sensitivity['stdev_cost'] > sensitivity['stdev_cost'].quantile(0.75)]
    print(f"  High-uncertainty routes: {len(high_uncertainty)}")
    print(f"  Top 3 uncertain routes:")
    print(high_uncertainty.nlargest(3, 'stdev_cost')[['origin_id', 'destination_id', 'baseline_cost', 'stdev_cost']])
    
    return result


def cookbook_stream_d_od_sensitivity_route_robustness():
    """
    Stream D: Route Robustness Analysis - Compare route options under uncertainty
    
    Scenario: Logistics company plans routes but wants to evaluate how robust each option
    is to traffic variation. Multiple path options evaluated for cost stability.
    """
    wbe = WbEnvironment()
    
    # Monte Carlo with higher sampling for robust analysis
    result = wbe.run_tool(
        "od_sensitivity_analysis",
        {
            "input": "data/multipath_network.gpkg",
            "origins": "data/shipper_origin.gpkg",
            "destinations": "data/shipper_destination.gpkg",
            "edge_cost_field": "TRAVEL_TIME_MIN",
            "impedance_disturbance_range": "0.9,1.1",  # ±10% variation
            "monte_carlo_samples": 50,
            "max_snap_distance": 100.0,
            "output": "output/route_robustness.csv"
        }
    )
    
    import pandas as pd
    robustness = pd.read_csv("output/route_robustness.csv")
    
    # Calculate coefficient of variation (robustness metric)
    robustness['cv'] = robustness['stdev_cost'] / robustness['mean_cost']
    
    print(f"Route Robustness Analysis:")
    print(f"  Baseline cost: {robustness['baseline_cost'].values[0]:.2f} min")
    print(f"  Mean cost (Monte Carlo): {robustness['mean_cost'].values[0]:.2f} min")
    print(f"  Cost uncertainty (stdev): {robustness['stdev_cost'].values[0]:.2f} min")
    print(f"  Coefficient of variation: {robustness['cv'].values[0]:.3f}")
    print(f"  Cost range: {robustness['min_cost'].values[0]:.2f} - {robustness['max_cost'].values[0]:.2f} min")
    
    return result


if __name__ == "__main__":
    print("=" * 70)
    print("Stream D Network Analytics Cookbook")
    print("=" * 70)
    
    print("\n[Stream D] Network Centrality - Urban Planning")
    print("-" * 70)
    cookbook_stream_d_centrality_urban_planning()
    
    print("\n[Stream D] Accessibility Metrics - Retail Site Selection")
    print("-" * 70)
    cookbook_stream_d_accessibility_retail_site_selection()
    
    print("\n[Stream D] Accessibility Metrics - Healthcare Equity")
    print("-" * 70)
    cookbook_stream_d_accessibility_healthcare_equity()
    
    print("\n[Stream D] OD Sensitivity - Freight Cost Uncertainty")
    print("-" * 70)
    cookbook_stream_d_od_sensitivity_cost_uncertainty()
    
    print("\n[Stream D] OD Sensitivity - Route Robustness Analysis")
    print("-" * 70)
    cookbook_stream_d_od_sensitivity_route_robustness()
    
    print("\n" + "=" * 70)
    print("Stream D cookbook examples complete!")
    print("=" * 70)
