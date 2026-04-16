from .whitebox_workflows import *
from . import callbacks
from .callbacks import ProgressPrinter, make_progress_printer, print_progress


def _add_if_not_none(args, key, value):
    if value is not None:
        args[key] = value


def analyze_multimodal_od_scenarios(
    env,
    input,
    origins,
    destinations,
    output,
    mode_field="MODE",
    allowed_modes=None,
    mode_speed_overrides=None,
    transfer_penalty=None,
    edge_cost_field=None,
    max_snap_distance=None,
    scenario_bundle_csv=None,
    temporal_cost_profile=None,
    departure_time=None,
    temporal_mode=None,
    parallel_execution=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
        "mode_field": mode_field,
    }
    _add_if_not_none(args, "allowed_modes", allowed_modes)
    _add_if_not_none(args, "mode_speed_overrides", mode_speed_overrides)
    _add_if_not_none(args, "transfer_penalty", transfer_penalty)
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "scenario_bundle_csv", scenario_bundle_csv)
    _add_if_not_none(args, "temporal_cost_profile", temporal_cost_profile)
    _add_if_not_none(args, "departure_time", departure_time)
    _add_if_not_none(args, "temporal_mode", temporal_mode)
    _add_if_not_none(args, "parallel_execution", parallel_execution)
    return env.run_tool("multimodal_od_cost_matrix", args)


def export_multimodal_routes_for_od_pairs(
    env,
    input,
    origins,
    destinations,
    output,
    mode_field="MODE",
    allowed_modes=None,
    mode_speed_overrides=None,
    transfer_penalty=None,
    edge_cost_field=None,
    max_snap_distance=None,
    scenario_bundle_csv=None,
    temporal_cost_profile=None,
    departure_time=None,
    temporal_mode=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
        "mode_field": mode_field,
    }
    _add_if_not_none(args, "allowed_modes", allowed_modes)
    _add_if_not_none(args, "mode_speed_overrides", mode_speed_overrides)
    _add_if_not_none(args, "transfer_penalty", transfer_penalty)
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "scenario_bundle_csv", scenario_bundle_csv)
    _add_if_not_none(args, "temporal_cost_profile", temporal_cost_profile)
    _add_if_not_none(args, "departure_time", departure_time)
    _add_if_not_none(args, "temporal_mode", temporal_mode)
    return env.run_tool("multimodal_routes_from_od", args)


def compute_network_accessibility(
    env,
    input,
    origins,
    destinations,
    output,
    edge_cost_field=None,
    max_snap_distance=None,
    impedance_cutoff=None,
    decay_function=None,
    decay_parameter=None,
    parallel_execution=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
    }
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "impedance_cutoff", impedance_cutoff)
    _add_if_not_none(args, "decay_function", decay_function)
    _add_if_not_none(args, "decay_parameter", decay_parameter)
    _add_if_not_none(args, "parallel_execution", parallel_execution)
    return env.run_tool("network_accessibility_metrics", args)


def analyze_od_cost_sensitivity(
    env,
    input,
    origins,
    destinations,
    output,
    edge_cost_field=None,
    max_snap_distance=None,
    impedance_disturbance_range=None,
    monte_carlo_samples=None,
    parallel_execution=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
    }
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "impedance_disturbance_range", impedance_disturbance_range)
    _add_if_not_none(args, "monte_carlo_samples", monte_carlo_samples)
    _add_if_not_none(args, "parallel_execution", parallel_execution)
    return env.run_tool("od_sensitivity_analysis", args)


def lidar_change_and_disturbance_analysis(
    env,
    baseline_tiles,
    monitor_tiles,
    resolution=None,
    min_change_m=None,
    output_prefix=None,
):
    args = {
        "baseline_tiles": baseline_tiles,
        "monitor_tiles": monitor_tiles,
    }
    _add_if_not_none(args, "resolution", resolution)
    _add_if_not_none(args, "min_change_m", min_change_m)
    _add_if_not_none(args, "output_prefix", output_prefix)
    return env.run_tool("lidar_change_and_disturbance_analysis", args)


def sidewalk_vegetation_accessibility_monitoring(
    env,
    lidar_tiles,
    sidewalks,
    sidewalks_epsg=None,
    resolution=None,
    segment_length_m=None,
    clearance_height_m=None,
    buffer_distance_m=None,
    output_prefix=None,
):
    args = {
        "lidar_tiles": lidar_tiles,
        "sidewalks": sidewalks,
    }
    _add_if_not_none(args, "sidewalks_epsg", sidewalks_epsg)
    _add_if_not_none(args, "resolution", resolution)
    _add_if_not_none(args, "segment_length_m", segment_length_m)
    _add_if_not_none(args, "clearance_height_m", clearance_height_m)
    _add_if_not_none(args, "buffer_distance_m", buffer_distance_m)
    _add_if_not_none(args, "output_prefix", output_prefix)
    return env.run_tool("sidewalk_vegetation_accessibility_monitoring", args)


def terrain_constraint_and_conflict_analysis(
    env,
    dem,
    wetness=None,
    flood_risk=None,
    landcover_penalty=None,
    slope_limit_deg=None,
    output_prefix=None,
):
    args = {
        "dem": dem,
    }
    _add_if_not_none(args, "wetness", wetness)
    _add_if_not_none(args, "flood_risk", flood_risk)
    _add_if_not_none(args, "landcover_penalty", landcover_penalty)
    _add_if_not_none(args, "slope_limit_deg", slope_limit_deg)
    _add_if_not_none(args, "output_prefix", output_prefix)
    return env.run_tool("terrain_constraint_and_conflict_analysis", args)


def terrain_constructability_and_cost_analysis(
    env,
    dem,
    existing_conflict=None,
    wetness=None,
    access_cost=None,
    output_prefix=None,
):
    args = {
        "dem": dem,
    }
    _add_if_not_none(args, "existing_conflict", existing_conflict)
    _add_if_not_none(args, "wetness", wetness)
    _add_if_not_none(args, "access_cost", access_cost)
    _add_if_not_none(args, "output_prefix", output_prefix)
    return env.run_tool("terrain_constructability_and_cost_analysis", args)


def in_season_crop_stress_intervention_planning(
    env,
    ndvi,
    canopy_temperature=None,
    soil_moisture=None,
    output_prefix=None,
):
    args = {
        "ndvi": ndvi,
    }
    _add_if_not_none(args, "canopy_temperature", canopy_temperature)
    _add_if_not_none(args, "soil_moisture", soil_moisture)
    _add_if_not_none(args, "output_prefix", output_prefix)
    return env.run_tool("in_season_crop_stress_intervention_planning", args)


def field_trafficability_and_operation_planning(
    env,
    dem,
    soil_moisture,
    rainfall_forecast=None,
    output_prefix=None,
):
    args = {
        "dem": dem,
        "soil_moisture": soil_moisture,
    }
    _add_if_not_none(args, "rainfall_forecast", rainfall_forecast)
    _add_if_not_none(args, "output_prefix", output_prefix)
    return env.run_tool("field_trafficability_and_operation_planning", args)


def _attach_phase4_convenience_methods():
    if "WbEnvironment" not in globals():
        return
    WbEnvironment.analyze_multimodal_od_scenarios = analyze_multimodal_od_scenarios
    WbEnvironment.export_multimodal_routes_for_od_pairs = export_multimodal_routes_for_od_pairs
    WbEnvironment.compute_network_accessibility = compute_network_accessibility
    WbEnvironment.analyze_od_cost_sensitivity = analyze_od_cost_sensitivity
    WbEnvironment.lidar_change_and_disturbance_analysis = lidar_change_and_disturbance_analysis
    WbEnvironment.sidewalk_vegetation_accessibility_monitoring = sidewalk_vegetation_accessibility_monitoring
    WbEnvironment.terrain_constraint_and_conflict_analysis = terrain_constraint_and_conflict_analysis
    WbEnvironment.terrain_constructability_and_cost_analysis = terrain_constructability_and_cost_analysis
    WbEnvironment.in_season_crop_stress_intervention_planning = in_season_crop_stress_intervention_planning
    WbEnvironment.field_trafficability_and_operation_planning = field_trafficability_and_operation_planning


_attach_phase4_convenience_methods()

__doc__ = whitebox_workflows.__doc__
if hasattr(whitebox_workflows, "__all__"):
    __all__ = list(whitebox_workflows.__all__) + [
        "callbacks",
        "ProgressPrinter",
        "make_progress_printer",
        "print_progress",
        "analyze_multimodal_od_scenarios",
        "export_multimodal_routes_for_od_pairs",
        "compute_network_accessibility",
        "analyze_od_cost_sensitivity",
        "lidar_change_and_disturbance_analysis",
        "sidewalk_vegetation_accessibility_monitoring",
        "terrain_constraint_and_conflict_analysis",
        "terrain_constructability_and_cost_analysis",
        "in_season_crop_stress_intervention_planning",
        "field_trafficability_and_operation_planning",
    ]
else:
    __all__ = [
        "callbacks",
        "ProgressPrinter",
        "make_progress_printer",
        "print_progress",
        "analyze_multimodal_od_scenarios",
        "export_multimodal_routes_for_od_pairs",
        "compute_network_accessibility",
        "analyze_od_cost_sensitivity",
        "lidar_change_and_disturbance_analysis",
        "sidewalk_vegetation_accessibility_monitoring",
        "terrain_constraint_and_conflict_analysis",
        "terrain_constructability_and_cost_analysis",
        "in_season_crop_stress_intervention_planning",
        "field_trafficability_and_operation_planning",
    ]
