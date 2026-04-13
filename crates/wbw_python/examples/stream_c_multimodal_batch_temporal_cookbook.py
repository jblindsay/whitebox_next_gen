"""
Stream C Cookbook: Batched multimodal OD and route analysis with temporal scenarios.

Any WbEnvironment instance name works; this example uses env for brevity.
"""

from pathlib import Path

import geopandas as gpd
import pandas as pd

from wbw import WbEnvironment


OUTPUT_DIR = Path("output")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)


def cookbook_multimodal_od_temporal_scenarios():
    """Compare peak-period and off-peak OD costs for a multimodal commute market."""
    env = WbEnvironment()

    result = env.run_tool(
        "multimodal_od_cost_matrix",
        {
            "input": "data/multimodal_network.gpkg",
            "origins": "data/commuter_origins.gpkg",
            "destinations": "data/employment_centers.gpkg",
            "mode_field": "MODE",
            "allowed_modes": "walk,transit,drive",
            "mode_speed_overrides": "walk:1.3,transit:7.5,drive:12.0",
            "transfer_penalty": 0.2,
            "scenario_bundle_csv": "data/multimodal_temporal_scenarios.csv",
            "output": str(OUTPUT_DIR / "multimodal_temporal_od.csv"),
        },
    )

    od = pd.read_csv(OUTPUT_DIR / "multimodal_temporal_od.csv")
    reachable = od[od["reachable"] == True].copy()
    summary = (
        reachable.groupby("scenario", dropna=False)["cost"]
        .agg(["count", "mean", "max"])
        .reset_index()
    )

    print("Temporal multimodal OD summary:")
    print(summary.to_string(index=False))
    print(f"Scenarios processed: {result['scenario_count']}")
    print(f"Reachable pairs: {result['reachable_pair_count']}")

    return result, od


def cookbook_multimodal_routes_temporal_comparison():
    """Generate comparable route geometries for multiple temporal scenarios."""
    env = WbEnvironment()

    result = env.run_tool(
        "multimodal_routes_from_od",
        {
            "input": "data/multimodal_network.gpkg",
            "origins": "data/key_origins.gpkg",
            "destinations": "data/key_destinations.gpkg",
            "mode_field": "MODE",
            "allowed_modes": "walk,transit,drive",
            "mode_speed_overrides": "walk:1.3,transit:7.5,drive:12.0",
            "transfer_penalty": 0.2,
            "scenario_bundle_csv": "data/multimodal_temporal_scenarios.csv",
            "output": str(OUTPUT_DIR / "multimodal_temporal_routes.gpkg"),
        },
    )

    routes = gpd.read_file(OUTPUT_DIR / "multimodal_temporal_routes.gpkg")
    route_summary = (
        routes.groupby("SCENARIO", dropna=False)
        .agg(route_count=("COST", "size"), mean_cost=("COST", "mean"))
        .reset_index()
    )

    print("Temporal multimodal route summary:")
    print(route_summary.to_string(index=False))
    print(f"Routes written: {result['route_count']}")

    return result, routes


if __name__ == "__main__":
    print("=" * 72)
    print("Stream C Multimodal Batch Temporal Cookbook")
    print("=" * 72)
    cookbook_multimodal_od_temporal_scenarios()
    print("-" * 72)
    cookbook_multimodal_routes_temporal_comparison()
